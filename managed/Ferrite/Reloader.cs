using System;
using System.IO;
using System.Reflection;
using Mono.Cecil;

namespace Ferrite {
    static class Reloader {
        static int generation;

        internal static Type Load(string path, string typeName) {
            var resolver = new DefaultAssemblyResolver();
            resolver.AddSearchDirectory(Path.GetDirectoryName(path));
            resolver.AddSearchDirectory(Bootstrap.CoreDir);
            var assembly = AssemblyDefinition.ReadAssembly(new MemoryStream(File.ReadAllBytes(path)), new ReaderParameters { AssemblyResolver = resolver });
            generation++;
            assembly.Name.Name = $"{assembly.Name.Name}-reload{generation}";
            assembly.MainModule.Name = assembly.Name.Name + ".dll";
            var output = new MemoryStream();
            assembly.Write(output);
            return Assembly.Load(output.ToArray()).GetType(typeName, false);
        }
    }
}
