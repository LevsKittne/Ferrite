using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text.RegularExpressions;
using BepInEx.Logging;
using Mono.Cecil;
using UnityEngine;

namespace BepInEx.Bootstrap {
    public static class Chainloader {
        static readonly List<BaseUnityPlugin> plugins = new List<BaseUnityPlugin>();
        static readonly Dictionary<string, string> libraries = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        static readonly HashSet<string> faulted = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        static readonly Ferrite.Logger log = new Ferrite.Logger("BepInEx");
        static bool started;

        public static Dictionary<string, PluginInfo> PluginInfos { get; } = new Dictionary<string, PluginInfo>();

        [Obsolete("Use PluginInfos instead")]
        public static List<BaseUnityPlugin> Plugins {
            get {
                lock (plugins) {
                    plugins.RemoveAll(x => x == null);
                    return plugins.ToList();
                }
            }
        }

        public static List<string> DependencyErrors { get; } = new List<string>();

        public static GameObject ManagerObject { get; private set; }

        public static void Initialize(string gameExePath, bool startConsole = true, ICollection<LogEventArgs> preloaderLogEvents = null) {
        }

        public static void Start() {
            if (started) {
                return;
            }
            started = true;
            var clock = Stopwatch.StartNew();
            Directory.CreateDirectory(Paths.PluginPath);
            var dirs = new List<string> { Paths.PluginPath };
            var bepinex = Path.Combine(Path.Combine(Paths.GameRootPath, "BepInEx"), "plugins");
            if (Directory.Exists(bepinex)) {
                dirs.Add(bepinex);
            }
            AppDomain.CurrentDomain.AssemblyResolve += Resolve;
            var candidates = Scan(dirs);
            ManagerObject = new GameObject("BepInEx_Manager") { hideFlags = HideFlags.HideAndDontSave };
            UnityEngine.Object.DontDestroyOnLoad(ManagerObject);
            ThreadingHelper.Initialize(ManagerObject);
            var loaded = 0;
            foreach (var candidate in Order(candidates)) {
                try {
                    if (Load(candidate)) {
                        loaded++;
                    }
                } catch (Exception e) {
                    PluginInfos.Remove(candidate.Guid);
                    log.Error($"{candidate} failed to load: {Describe(e)}");
                }
            }
            Ferrite.Commands.Register("plugins", "Lists BepInEx plugins.", _ => {
                Ferrite.Commands.Print($"{PluginInfos.Count} BepInEx plugin(s):");
                foreach (var info in PluginInfos.Values) {
                    Ferrite.Commands.Print($"  {info.Metadata.Name} {info.Metadata.Version} ({info.Metadata.GUID}){(faulted.Contains(info.Metadata.GUID) ? " [faulted]" : "")} {info.Location}");
                }
                foreach (var error in DependencyErrors) {
                    Ferrite.Commands.Print("  " + error);
                }
            });
            Report();
            log.Info($"{loaded} BepInEx plugin(s) loaded in {clock.ElapsedMilliseconds} ms");
        }

        sealed class Candidate {
            public string Guid;
            public string Name;
            public string VersionText;
            public Version Version;
            public string TypeName;
            public string File;
            public readonly List<(string Guid, bool Hard, Version Min)> Dependencies = new List<(string, bool, Version)>();
            public readonly List<string> Incompatibilities = new List<string>();
            public readonly List<string> Processes = new List<string>();

            public override string ToString() => $"{Name} {VersionText} ({Guid})";
        }

        static List<Candidate> Scan(List<string> dirs) {
            var resolver = new DefaultAssemblyResolver();
            foreach (var dir in new[] { Paths.BepInExAssemblyDirectory, Paths.ManagedPath }.Concat(dirs).Concat(dirs.SelectMany(d => Directory.GetDirectories(d, "*", SearchOption.AllDirectories)))) {
                resolver.AddSearchDirectory(dir);
            }
            var parameters = new ReaderParameters { AssemblyResolver = resolver };
            var loadedNames = new HashSet<string>(AppDomain.CurrentDomain.GetAssemblies().Select(a => a.GetName().Name), StringComparer.OrdinalIgnoreCase);
            var result = new List<Candidate>();
            using (resolver) {
                foreach (var file in dirs.SelectMany(d => Directory.GetFiles(d, "*.dll", SearchOption.AllDirectories))) {
                    try {
                        result.AddRange(Scan(file, parameters, loadedNames));
                    } catch (Exception e) {
                        log.Error($"cannot read plugins from {file}: {e.Message}");
                    }
                }
            }
            return result;
        }

        static List<Candidate> Scan(string file, ReaderParameters parameters, HashSet<string> loadedNames) {
            var result = new List<Candidate>();
            AssemblyDefinition assembly;
            try {
                assembly = AssemblyDefinition.ReadAssembly(new MemoryStream(File.ReadAllBytes(file)), parameters);
            } catch (BadImageFormatException) {
                log.Debug($"skipping {file}: not a .NET assembly");
                return result;
            }
            using (assembly) {
                var name = assembly.Name.Name;
                if (loadedNames.Contains(name)) {
                    log.Debug($"skipping {file}: {name} is already loaded");
                    return result;
                }
                if (!libraries.ContainsKey(name)) {
                    libraries[name] = file;
                }
                foreach (var type in assembly.MainModule.Types) {
                    if (!type.IsClass || type.IsAbstract || type.IsInterface) {
                        continue;
                    }
                    var plugin = type.CustomAttributes.FirstOrDefault(a => a.AttributeType.FullName == "BepInEx.BepInPlugin");
                    if (plugin == null || plugin.ConstructorArguments.Count != 3) {
                        continue;
                    }
                    var c = new Candidate {
                        Guid = plugin.ConstructorArguments[0].Value as string,
                        Name = plugin.ConstructorArguments[1].Value as string,
                        VersionText = plugin.ConstructorArguments[2].Value as string,
                        TypeName = type.FullName,
                        File = file,
                    };
                    if (string.IsNullOrEmpty(c.Guid)) {
                        log.Error($"{type.FullName} in {file} has an empty GUID, skipped");
                        continue;
                    }
                    c.Name = string.IsNullOrEmpty(c.Name) ? c.Guid : c.Name;
                    c.Version = BepInPlugin.ParseVersion(c.VersionText) ?? new Version();
                    foreach (var attr in type.CustomAttributes) {
                        var args = attr.ConstructorArguments;
                        switch (attr.AttributeType.FullName) {
                            case "BepInEx.BepInDependency":
                                var guid = args[0].Value as string;
                                if (args.Count > 1 && args[1].Value is string min) {
                                    c.Dependencies.Add((guid, true, BepInPlugin.ParseVersion(min) ?? new Version()));
                                } else {
                                    var flags = args.Count > 1 ? Convert.ToInt32(args[1].Value) : 1;
                                    c.Dependencies.Add((guid, (flags & 1) != 0, new Version()));
                                }
                                break;
                            case "BepInEx.BepInIncompatibility":
                                c.Incompatibilities.Add(args[0].Value as string);
                                break;
                            case "BepInEx.BepInProcess":
                                c.Processes.Add(args[0].Value as string);
                                break;
                        }
                    }
                    result.Add(c);
                }
            }
            return result;
        }

        static List<Candidate> Order(List<Candidate> all) {
            var disabled = Disabled();
            var process = Paths.ProcessName;
            var byGuid = new Dictionary<string, Candidate>(StringComparer.OrdinalIgnoreCase);
            foreach (var c in all) {
                if (disabled.Contains(c.Guid)) {
                    log.Info($"{c} is disabled");
                    continue;
                }
                if (c.Processes.Count > 0 && !c.Processes.Any(p => string.Equals(Path.GetFileNameWithoutExtension(p ?? ""), process, StringComparison.OrdinalIgnoreCase))) {
                    log.Info($"skipping {c}: only for {string.Join(", ", c.Processes.ToArray())}");
                    continue;
                }
                if (byGuid.TryGetValue(c.Guid, out var other)) {
                    var keep = c.Version > other.Version ? c : other;
                    var drop = keep == c ? other : c;
                    log.Warning($"duplicate plugin {c.Guid}: using {keep.File}, skipping {drop.File}");
                    byGuid[c.Guid] = keep;
                    continue;
                }
                byGuid[c.Guid] = c;
            }
            var sorted = new List<Candidate>();
            var state = new Dictionary<Candidate, int>();
            var cyclic = new HashSet<Candidate>();
            void Visit(Candidate c, Stack<Candidate> path) {
                if (state.TryGetValue(c, out var s)) {
                    if (s == 1) {
                        foreach (var p in path.TakeWhile(p => p != c)) {
                            cyclic.Add(p);
                        }
                        cyclic.Add(c);
                    }
                    return;
                }
                state[c] = 1;
                path.Push(c);
                foreach (var dep in c.Dependencies) {
                    if (dep.Guid != null && byGuid.TryGetValue(dep.Guid, out var target)) {
                        Visit(target, path);
                    }
                }
                path.Pop();
                state[c] = 2;
                sorted.Add(c);
            }
            foreach (var c in byGuid.Values.OrderBy(c => c.Guid, StringComparer.OrdinalIgnoreCase)) {
                Visit(c, new Stack<Candidate>());
            }
            var result = new List<Candidate>();
            var accepted = new Dictionary<string, Version>(StringComparer.OrdinalIgnoreCase);
            foreach (var c in sorted) {
                string error = null;
                if (cyclic.Contains(c)) {
                    error = "is part of a dependency cycle";
                } else {
                    var missing = c.Dependencies.Where(d => d.Hard && (!accepted.TryGetValue(d.Guid ?? "", out var v) || v < d.Min)).ToList();
                    if (missing.Count > 0) {
                        error = "is missing dependencies: " + string.Join(", ", missing.Select(d => d.Min > new Version() ? $"{d.Guid} v{d.Min} or newer" : d.Guid).ToArray());
                    } else {
                        var incompatible = c.Incompatibilities.Where(i => i != null && byGuid.ContainsKey(i)).ToList();
                        if (incompatible.Count > 0) {
                            error = "is incompatible with " + string.Join(", ", incompatible.ToArray());
                        }
                    }
                }
                if (error != null) {
                    var message = $"Could not load [{c}] because it {error}";
                    DependencyErrors.Add(message);
                    log.Error(message);
                    continue;
                }
                accepted[c.Guid] = c.Version;
                result.Add(c);
            }
            return result;
        }

        static bool Load(Candidate c) {
            var broken = c.Dependencies.Where(d => d.Hard && d.Guid != null && !PluginInfos.ContainsKey(d.Guid)).Select(d => d.Guid).ToList();
            if (broken.Count > 0) {
                var message = $"Could not load [{c}] because it depends on plugins that failed to load: {string.Join(", ", broken.ToArray())}";
                DependencyErrors.Add(message);
                log.Error(message);
                return false;
            }
            Type type;
            try {
                type = Assembly.LoadFrom(c.File).GetType(c.TypeName, true);
            } catch (Exception e) {
                log.Error($"{c} failed to load from {c.File}: {Describe(e)}");
                return false;
            }
            if (!typeof(BaseUnityPlugin).IsAssignableFrom(type)) {
                log.Error($"{c}: {c.TypeName} does not inherit BepInEx.BaseUnityPlugin, skipped");
                return false;
            }
            PluginInfo info;
            try {
                info = MetadataHelper.Describe(type, c.File);
            } catch (Exception e) {
                log.Error($"{c} has broken attributes: {Describe(e)}");
                return false;
            }
            PluginInfos[c.Guid] = info;
            var errors = new List<string>();
            void Capture(string message, string trace, LogType kind) {
                if (kind == LogType.Exception) {
                    errors.Add(message);
                }
            }
            Application.logMessageReceived += Capture;
            Component component = null;
            Exception thrown = null;
            try {
                component = ManagerObject.AddComponent(type);
            } catch (Exception e) {
                thrown = e;
            } finally {
                Application.logMessageReceived -= Capture;
            }
            var plugin = component as BaseUnityPlugin;
            if (plugin == null) {
                PluginInfos.Remove(c.Guid);
                log.Error($"{c} failed to start: {(thrown != null ? Describe(thrown) : errors.FirstOrDefault() ?? "component was not created")}");
                return false;
            }
            info.Instance = plugin;
            lock (plugins) {
                plugins.Add(plugin);
            }
            if (errors.Count > 0 || thrown != null) {
                faulted.Add(c.Guid);
                log.Error($"{c} threw while starting: {(thrown != null ? Describe(thrown) : errors[0])}");
            } else {
                log.Info($"loaded {c.Name} {c.VersionText} ({c.Guid})");
            }
            return true;
        }

        static string Describe(Exception e) {
            if (e is TargetInvocationException && e.InnerException != null) {
                e = e.InnerException;
            }
            if (e is ReflectionTypeLoadException rtle) {
                return string.Join("; ", rtle.LoaderExceptions.Where(x => x != null).Select(x => x.Message).Distinct().ToArray());
            }
            return e.GetType().Name + ": " + e.Message;
        }

        static Assembly Resolve(object sender, ResolveEventArgs args) {
            var name = new AssemblyName(args.Name).Name;
            if (!libraries.TryGetValue(name, out var file)) {
                return null;
            }
            try {
                return Assembly.LoadFrom(file);
            } catch (Exception e) {
                log.Error($"cannot load {file}: {e.Message}");
                return null;
            }
        }

        static HashSet<string> Disabled() {
            var set = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
            try {
                var text = File.ReadAllText(Path.Combine(Ferrite.Runtime.RootDir, "ferrite.toml"));
                var match = Regex.Match(text, @"^\s*disabled\s*=\s*\[(.*?)\]", RegexOptions.Multiline | RegexOptions.Singleline);
                if (match.Success) {
                    foreach (Match m in Regex.Matches(match.Groups[1].Value, "\"((?:[^\"\\\\]|\\\\.)*)\"|'([^']*)'")) {
                        set.Add(m.Groups[1].Success ? m.Groups[1].Value : m.Groups[2].Value);
                    }
                }
            } catch (Exception) {
            }
            return set;
        }

        static void Report() {
            var rows = PluginInfos.Values
                .Where(i => i.Instance != null)
                .Select(i => new[] { i.Metadata.GUID, i.Metadata.Name, i.Metadata.Version?.ToString() ?? "", "", faulted.Contains(i.Metadata.GUID) ? "1" : "0" })
                .ToList();
            Ferrite.Runtime.ReportExternal("BepInEx", rows);
        }
    }
}
