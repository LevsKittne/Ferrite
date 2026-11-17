using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using UnityEngine;

namespace BepInEx {
    public interface IInputSystem {
        Vector3 mousePosition { get; }
        Vector2 mouseScrollDelta { get; }
        bool mousePresent { get; }
        bool anyKey { get; }
        bool anyKeyDown { get; }
        IEnumerable<KeyCode> SupportedKeyCodes { get; }

        bool GetKey(string name);
        bool GetKey(KeyCode key);
        bool GetKeyDown(string name);
        bool GetKeyDown(KeyCode key);
        bool GetKeyUp(string name);
        bool GetKeyUp(KeyCode key);
        bool GetMouseButton(int button);
        bool GetMouseButtonDown(int button);
        bool GetMouseButtonUp(int button);
        void ResetInputAxes();
    }

    public static class UnityInput {
        public static IInputSystem Current { get; } = new LegacyInputSystem();

        public static bool LegacyInputSystemAvailable => true;

        sealed class LegacyInputSystem : IInputSystem {
            static readonly KeyCode[] keyCodes = ((KeyCode[])Enum.GetValues(typeof(KeyCode))).Distinct().ToArray();

            public Vector3 mousePosition => Input.mousePosition;
            public Vector2 mouseScrollDelta => Input.mouseScrollDelta;
            public bool mousePresent => Input.mousePresent;
            public bool anyKey => Input.anyKey;
            public bool anyKeyDown => Input.anyKeyDown;
            public IEnumerable<KeyCode> SupportedKeyCodes => keyCodes;

            public bool GetKey(string name) => Input.GetKey(name);
            public bool GetKey(KeyCode key) => Input.GetKey(key);
            public bool GetKeyDown(string name) => Input.GetKeyDown(name);
            public bool GetKeyDown(KeyCode key) => Input.GetKeyDown(key);
            public bool GetKeyUp(string name) => Input.GetKeyUp(name);
            public bool GetKeyUp(KeyCode key) => Input.GetKeyUp(key);
            public bool GetMouseButton(int button) => Input.GetMouseButton(button);
            public bool GetMouseButtonDown(int button) => Input.GetMouseButtonDown(button);
            public bool GetMouseButtonUp(int button) => Input.GetMouseButtonUp(button);
            public void ResetInputAxes() => Input.ResetInputAxes();
        }
    }

    public sealed class ThreadingHelper : MonoBehaviour {
        readonly object queueLock = new object();
        Action queue;
        Thread mainThread;

        public static ThreadingHelper Instance { get; private set; }

        internal static void Initialize(GameObject host) {
            Instance = host.AddComponent<ThreadingHelper>();
        }

        void Awake() {
            mainThread = Thread.CurrentThread;
        }

        public bool InvokeRequired => mainThread == null || mainThread != Thread.CurrentThread;

        public void StartSyncInvoke(Action action) {
            if (action == null) {
                throw new ArgumentNullException(nameof(action));
            }
            lock (queueLock) {
                queue += action;
            }
        }

        public void StartAsyncInvoke(Func<Action> action) {
            if (action == null) {
                throw new ArgumentNullException(nameof(action));
            }
            ThreadPool.QueueUserWorkItem(_ => {
                try {
                    var result = action();
                    if (result != null) {
                        StartSyncInvoke(result);
                    }
                } catch (Exception e) {
                    Logging.Logger.Log(Logging.LogLevel.Error, "Unhandled exception in async invoke: " + e);
                }
            });
        }

        void Update() {
            Action pending;
            lock (queueLock) {
                pending = queue;
                queue = null;
            }
            if (pending == null) {
                return;
            }
            foreach (Action action in pending.GetInvocationList()) {
                try {
                    action();
                } catch (Exception e) {
                    Logging.Logger.Log(Logging.LogLevel.Error, "Unhandled exception in sync invoke: " + e);
                }
            }
        }
    }
}
