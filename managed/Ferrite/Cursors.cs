using UnityEngine;

namespace Ferrite {
    static class Cursors {
        static bool open;
        static CursorLockMode savedLock;
        static bool savedVisible;

        internal static void Update() {
            var now = Native.MenuOpen();
            if (now && !open) {
                savedLock = Cursor.lockState;
                savedVisible = Cursor.visible;
            } else if (!now && open) {
                Cursor.lockState = savedLock;
                Cursor.visible = savedVisible;
            }
            open = now;
            if (open) {
                Cursor.lockState = CursorLockMode.None;
                Cursor.visible = true;
            }
        }
    }
}
