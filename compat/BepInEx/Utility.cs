using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text;
using Mono.Cecil;

namespace BepInEx {
    public static class Utility {
        public static bool CLRSupportsDynamicAssemblies => true;

        public static Encoding UTF8NoBom { get; } = new UTF8Encoding(false);

        public static bool TryDo(Action action, out Exception exception) {
            exception = null;
            try {
                action();
                return true;
            } catch (Exception e) {
                exception = e;
                return false;
            }
        }

        public static string CombinePaths(params string[] parts) => parts.Aggregate(Path.Combine);

        public static string ParentDirectory(string path, int levels = 1) {
            for (var i = 0; i < levels; i++) {
                path = Path.GetDirectoryName(path);
            }
            return path;
        }

        public static bool SafeParseBool(string input, bool defaultValue = false) => bool.TryParse(input, out var result) ? result : defaultValue;

        public static string ConvertToWWWFormat(string path) => $"file://{path.Replace('\\', '/')}";

        public static bool IsNullOrWhiteSpace(this string self) => self == null || self.All(char.IsWhiteSpace);

        public static IEnumerable<TNode> TopologicalSort<TNode>(IEnumerable<TNode> nodes, Func<TNode, IEnumerable<TNode>> dependencySelector) {
            var sorted = new List<TNode>();
            var visited = new HashSet<TNode>();
            var done = new HashSet<TNode>();
            bool Visit(TNode node, Stack<TNode> stack) {
                if (visited.Contains(node)) {
                    return done.Contains(node);
                }
                visited.Add(node);
                stack.Push(node);
                foreach (var dep in dependencySelector(node)) {
                    if (!Visit(dep, stack)) {
                        return false;
                    }
                }
                done.Add(node);
                sorted.Add(node);
                stack.Pop();
                return true;
            }
            foreach (var node in nodes) {
                var stack = new Stack<TNode>();
                if (!Visit(node, stack)) {
                    throw new Exception("Cyclic Dependency:\r\n" + string.Join("\r\n", stack.Select(x => $" - {x}").ToArray()));
                }
            }
            return sorted;
        }

        public static bool IsSubtypeOf(this TypeDefinition self, Type td) {
            if (self.FullName == td.FullName) {
                return true;
            }
            return self.FullName != "System.Object" && (self.BaseType?.Resolve()?.IsSubtypeOf(td) ?? false);
        }

        public static bool TryResolveDllAssembly(AssemblyName assemblyName, string directory, out Assembly assembly) {
            assembly = null;
            var path = Find(assemblyName, directory);
            if (path == null) {
                return false;
            }
            try {
                assembly = Assembly.LoadFile(path);
                return true;
            } catch (Exception) {
                return false;
            }
        }

        public static bool TryResolveDllAssembly(AssemblyName assemblyName, string directory, ReaderParameters readerParameters, out AssemblyDefinition assembly) {
            assembly = null;
            var path = Find(assemblyName, directory);
            if (path == null) {
                return false;
            }
            try {
                assembly = AssemblyDefinition.ReadAssembly(path, readerParameters);
                return true;
            } catch (Exception) {
                return false;
            }
        }

        static string Find(AssemblyName name, string directory) {
            if (!Directory.Exists(directory)) {
                return null;
            }
            foreach (var dir in new[] { directory }.Concat(Directory.GetDirectories(directory, "*", SearchOption.AllDirectories))) {
                foreach (var ext in new[] { ".dll", ".exe" }) {
                    var path = Path.Combine(dir, name.Name + ext);
                    if (File.Exists(path)) {
                        return path;
                    }
                }
            }
            return null;
        }

        public static bool TryOpenFileStream(string path, FileMode mode, out FileStream fileStream, FileAccess access = FileAccess.ReadWrite, FileShare share = FileShare.Read) {
            try {
                fileStream = new FileStream(path, mode, access, share);
                return true;
            } catch (IOException) {
                fileStream = null;
                return false;
            }
        }

        public static bool TryParseAssemblyName(string fullName, out AssemblyName assemblyName) {
            try {
                assemblyName = new AssemblyName(fullName);
                return true;
            } catch (Exception) {
                assemblyName = null;
                return false;
            }
        }

        public static IEnumerable<string> GetUniqueFilesInDirectories(IEnumerable<string> directories, string pattern = "*") {
            var files = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            foreach (var dir in directories) {
                if (!Directory.Exists(dir)) {
                    continue;
                }
                foreach (var file in Directory.GetFiles(dir, pattern)) {
                    var name = Path.GetFileName(file);
                    if (!files.ContainsKey(name)) {
                        files[name] = file;
                    }
                }
            }
            return files.Values;
        }
    }
}
