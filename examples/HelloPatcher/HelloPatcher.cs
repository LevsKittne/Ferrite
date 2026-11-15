using System.Collections.Generic;
using Ferrite;
using Mono.Cecil;

namespace HelloPatcher {
    public sealed class HelloPatcher : Patcher {
        public override IEnumerable<string> Targets => new[] { "Assembly-CSharp.dll" };

        public override void Patch(AssemblyDefinition assembly) {
            var module = assembly.MainModule;
            var marker = new TypeDefinition("Ferrite", "PatchedByHelloPatcher", TypeAttributes.Public | TypeAttributes.Abstract | TypeAttributes.Sealed, module.TypeSystem.Object);
            module.Types.Add(marker);
            Logger.Info($"added Ferrite.PatchedByHelloPatcher to {assembly.Name.Name} ({module.Types.Count} types)");
        }
    }
}
