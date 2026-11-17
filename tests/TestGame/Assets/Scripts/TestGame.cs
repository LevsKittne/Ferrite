using UnityEngine;
using UnityEngine.UI;

public class TestGame : MonoBehaviour {
    public Transform cube;
    public Text label;

    float fps;
    float elapsed;
    int frames;
    GUIStyle style;

    void Start() {
        Debug.Log("TestGame graphics API: " + SystemInfo.graphicsDeviceType + " (" + SystemInfo.graphicsDeviceVersion + ")");
    }

    void Update() {
        if (cube != null) {
            cube.Rotate(new Vector3(20f, 45f, 10f) * Time.deltaTime);
        }
        frames++;
        elapsed += Time.unscaledDeltaTime;
        if (elapsed >= 0.5f) {
            fps = frames / elapsed;
            frames = 0;
            elapsed = 0f;
        }
        if (label != null) {
            label.text = "uGUI: " + SystemInfo.graphicsDeviceType;
        }
    }

    void OnGUI() {
        if (style == null) {
            style = new GUIStyle(GUI.skin.label) { fontSize = 28 };
            style.normal.textColor = Color.yellow;
        }
        GUI.Label(new Rect(20, Screen.height - 60, 900, 50), "IMGUI: " + SystemInfo.graphicsDeviceType + "  " + fps.ToString("0") + " FPS", style);
    }
}
