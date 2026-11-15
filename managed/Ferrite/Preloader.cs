using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using Mono.Cecil;
using Mono.Cecil.Cil;
using MethodAttributes = Mono.Cecil.MethodAttributes;

namespace Ferrite {
    public abstract class Patcher {
        public Logger Logger { get; internal set; }
        public string Location { get; internal set; }
        public abstract IEnumerable<string> Targets { get; }
        public abstract void Patch(AssemblyDefinition assembly);

        public virtual void Initialize() {
        }

        public virtual void Finish() {
        }
    }

    static class Preloader {
        static readonly Logger log = new Logger("Preloader");

        static string CacheDir => Path.Combine(Bootstrap.RootDir, "cache", "assemblies");

        static string EntryName(string managed) => File.Exists(Path.Combine(managed, "UnityEngine.CoreModule.dll")) ? "UnityEngine.CoreModule.dll" : "UnityEngine.dll";

        static string CacheKey(string path) {
            var info = new FileInfo(path);
            return $"{Runtime.Version}|{info.Length}|{info.LastWriteTimeUtc.Ticks}";
        }

        internal static void Run(bool early) {
            var managed = Native.Option("managed_dir");
            if (string.IsNullOrEmpty(managed) || !Directory.Exists(managed)) {
                log.Warning("managed folder not found, patchers and early start are disabled");
                return;
            }
            Directory.CreateDirectory(Bootstrap.PatchersDir);
            var hasPatchers = Directory.GetFiles(Bootstrap.PatchersDir, "*.dll", SearchOption.AllDirectories).Length > 0;
            if (!hasPatchers && early && Native.Option("dump_assemblies") != "true" && LoadCached(Path.Combine(managed, EntryName(managed)))) {
                return;
            }
            RunFull(early, managed);
        }

        static bool LoadCached(string path) {
            try {
                var name = Path.GetFileName(path);
                var key = Path.Combine(CacheDir, name + ".key");
                var data = Path.Combine(CacheDir, name);
                if (!File.Exists(key) || !File.Exists(data) || File.ReadAllText(key) != CacheKey(path)) {
                    return false;
                }
                var started = DateTime.Now;
                if (!Native.LoadPatched(path, File.ReadAllBytes(data))) {
                    return false;
                }
                log.Info($"entrypoint loaded from cache in {(DateTime.Now - started).TotalMilliseconds:0} ms");
                return true;
            } catch (Exception e) {
                log.Warning($"assembly cache unusable: {e.Message}");
                return false;
            }
        }

        static void SaveCached(string path, byte[] image) {
            try {
                Directory.CreateDirectory(CacheDir);
                var name = Path.GetFileName(path);
                File.WriteAllBytes(Path.Combine(CacheDir, name), image);
                File.WriteAllText(Path.Combine(CacheDir, name + ".key"), CacheKey(path));
            } catch (Exception e) {
                log.Warning($"cannot cache {Path.GetFileName(path)}: {e.Message}");
            }
        }

        [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.NoInlining)]
        static void RunFull(bool early, string managed) {
            var patchers = LoadPatchers(Bootstrap.PatchersDir);
            var targets = new Dictionary<string, List<Patcher>>(StringComparer.OrdinalIgnoreCase);
            foreach (var p in patchers) {
                foreach (var t in p.Targets ?? Enumerable.Empty<string>()) {
                    var name = t.EndsWith(".dll", StringComparison.OrdinalIgnoreCase) ? t : t + ".dll";
                    if (!targets.TryGetValue(name, out var list)) {
                        targets[name] = list = new List<Patcher>();
                    }
                    list.Add(p);
                }
            }
            string entry = null;
            if (early) {
                entry = EntryName(managed);
                if (!targets.ContainsKey(entry)) {
                    targets[entry] = new List<Patcher>();
                }
            }
            if (targets.Count == 0) {
                return;
            }
            var dump = Native.Option("dump_assemblies") == "true";
            var dumpDir = Path.Combine(Bootstrap.RootDir, "dump");
            var resolver = new DefaultAssemblyResolver();
            resolver.AddSearchDirectory(managed);
            resolver.AddSearchDirectory(Bootstrap.CoreDir);
            var started = DateTime.Now;
            var patched = 0;
            foreach (var pair in targets) {
                var path = Path.Combine(managed, pair.Key);
                if (!File.Exists(path)) {
                    log.Warning($"patch target {pair.Key} not found");
                    continue;
                }
                try {
                    if (PatchOne(path, pair.Value, pair.Key == entry, resolver, dump ? dumpDir : null)) {
                        patched++;
                    }
                } catch (Exception e) {
                    log.Error($"cannot patch {pair.Key}: {e}");
                }
            }
            foreach (var p in patchers) {
                try {
                    p.Finish();
                } catch (Exception e) {
                    log.Error($"{p.GetType().FullName}.Finish threw: {e}");
                }
            }
            log.Info($"{patchers.Count} patcher(s), {patched} assembly(ies) patched in {(DateTime.Now - started).TotalMilliseconds:0} ms");
        }

        static List<Patcher> LoadPatchers(string dir) {
            var result = new List<Patcher>();
            foreach (var file in Directory.GetFiles(dir, "*.dll", SearchOption.AllDirectories).OrderBy(f => f, StringComparer.OrdinalIgnoreCase)) {
                Type[] types;
                try {
                    types = Assembly.LoadFrom(file).GetTypes();
                } catch (ReflectionTypeLoadException e) {
                    types = e.Types.Where(t => t != null).ToArray();
                } catch (Exception e) {
                    log.Warning($"skipping {Path.GetFileName(file)}: {e.Message}");
                    continue;
                }
                foreach (var type in types.Where(t => !t.IsAbstract && typeof(Patcher).IsAssignableFrom(t))) {
                    try {
                        var p = (Patcher)Activator.CreateInstance(type);
                        p.Logger = new Logger(type.Name);
                        p.Location = file;
                        p.Initialize();
                        result.Add(p);
                        log.Info($"patcher {type.FullName} from {Path.GetFileName(file)}");
                    } catch (Exception e) {
                        log.Error($"patcher {type.FullName} failed to initialize: {e}");
                    }
                }
            }
            return result;
        }

        static bool PatchOne(string path, List<Patcher> patchers, bool entry, IAssemblyResolver resolver, string dumpDir) {
            var bytes = File.ReadAllBytes(path);
            var assembly = AssemblyDefinition.ReadAssembly(new MemoryStream(bytes), new ReaderParameters { AssemblyResolver = resolver });
            var changed = false;
            foreach (var p in patchers) {
                try {
                    p.Patch(assembly);
                    changed = true;
                } catch (Exception e) {
                    log.Error($"{p.GetType().FullName} failed on {Path.GetFileName(path)}: {e}");
                }
            }
            if (entry) {
                changed |= PatchEntrypoint(assembly);
            }
            if (!changed) {
                return false;
            }
            var output = new MemoryStream();
            assembly.Write(output);
            var image = output.ToArray();
            if (dumpDir != null) {
                Directory.CreateDirectory(dumpDir);
                File.WriteAllBytes(Path.Combine(dumpDir, Path.GetFileName(path)), image);
            }
            if (entry && patchers.Count == 0) {
                SaveCached(path, image);
            }
            return Native.LoadPatched(path, image);
        }

        static bool PatchEntrypoint(AssemblyDefinition assembly) {
            var type = assembly.MainModule.GetType("UnityEngine", "Application");
            if (type == null) {
                log.Warning($"UnityEngine.Application not found in {assembly.Name.Name}, mods start on the first scene");
                return false;
            }
            var module = assembly.MainModule;
            var ferrite = AssemblyNameReference.Parse(typeof(Preloader).Assembly.FullName);
            module.AssemblyReferences.Add(ferrite);
            var start = new MethodReference(nameof(Runtime.EarlyStart), module.TypeSystem.Void, new TypeReference("Ferrite", nameof(Runtime), module, ferrite));
            var cctor = type.Methods.FirstOrDefault(m => m.IsConstructor && m.IsStatic);
            if (cctor == null) {
                cctor = new MethodDefinition(".cctor", MethodAttributes.Private | MethodAttributes.HideBySig | MethodAttributes.SpecialName | MethodAttributes.RTSpecialName | MethodAttributes.Static, assembly.MainModule.TypeSystem.Void);
                cctor.Body.GetILProcessor().Emit(OpCodes.Ret);
                type.Methods.Add(cctor);
            }
            type.IsBeforeFieldInit = false;
            var il = cctor.Body.GetILProcessor();
            il.InsertBefore(cctor.Body.Instructions[0], il.Create(OpCodes.Call, start));
            return true;
        }
    }
}
