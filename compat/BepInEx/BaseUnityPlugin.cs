using System;
using System.IO;
using BepInEx.Bootstrap;
using BepInEx.Configuration;
using BepInEx.Logging;
using UnityEngine;

namespace BepInEx {
    public abstract class BaseUnityPlugin : MonoBehaviour {
        protected BaseUnityPlugin() {
            var metadata = MetadataHelper.GetMetadata(this);
            if (metadata == null) {
                throw new InvalidOperationException("Can't create an instance of " + GetType().FullName + " because it inherits from BaseUnityPlugin and the BepInPlugin attribute is missing.");
            }
            if (!Chainloader.PluginInfos.TryGetValue(metadata.GUID, out var info)) {
                info = MetadataHelper.Describe(GetType(), GetType().Assembly.Location);
                Chainloader.PluginInfos[metadata.GUID] = info;
            }
            info.Instance = this;
            Info = info;
            Logger = Logging.Logger.CreateLogSource(metadata.Name);
            Config = new ConfigFile(Path.Combine(Paths.ConfigPath, metadata.GUID + ".cfg"), false, metadata);
        }

        public PluginInfo Info { get; }

        protected ManualLogSource Logger { get; }

        public ConfigFile Config { get; }
    }
}
