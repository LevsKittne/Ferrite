using System;
using System.IO;
using UnityEditor;
using UnityEditor.Build;
using UnityEditor.Build.Reporting;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.Rendering;
using UnityEngine.UI;

public static class TestGameBuild {
    const string ScenePath = "Assets/Scenes/Main.unity";

    public static void BuildMono() {
        Build(ScriptingImplementation.Mono2x, Arg("-out", "D:/Projects/Ferrite-testgame/Mono"));
    }

    public static void BuildIl2Cpp() {
        Build(ScriptingImplementation.IL2CPP, Arg("-out", "D:/Projects/Ferrite-testgame/IL2CPP"));
    }

    static string Arg(string name, string fallback) {
        var args = Environment.GetCommandLineArgs();
        for (var i = 0; i < args.Length - 1; i++) {
            if (args[i] == name) {
                return args[i + 1];
            }
        }
        return fallback;
    }

    static void Build(ScriptingImplementation backend, string outDir) {
        Configure(backend);
        if (!File.Exists(ScenePath)) {
            CreateScene();
        }
        EditorBuildSettings.scenes = new[] { new EditorBuildSettingsScene(ScenePath, true) };
        Directory.CreateDirectory(outDir);
        var options = new BuildPlayerOptions {
            scenes = new[] { ScenePath },
            locationPathName = Path.Combine(outDir, "TestGame.exe"),
            target = BuildTarget.StandaloneWindows64,
            targetGroup = BuildTargetGroup.Standalone,
            options = BuildOptions.None
        };
        var report = BuildPipeline.BuildPlayer(options);
        Debug.Log("TestGame build " + backend + ": " + report.summary.result + " -> " + options.locationPathName);
        if (report.summary.result != BuildResult.Succeeded) {
            EditorApplication.Exit(1);
        }
    }

    static void Configure(ScriptingImplementation backend) {
        PlayerSettings.companyName = "Ferrite";
        PlayerSettings.productName = "TestGame";
        PlayerSettings.runInBackground = true;
        PlayerSettings.visibleInBackground = true;
        PlayerSettings.fullScreenMode = FullScreenMode.Windowed;
        PlayerSettings.defaultScreenWidth = 1280;
        PlayerSettings.defaultScreenHeight = 720;
        PlayerSettings.resizableWindow = true;
        PlayerSettings.usePlayerLog = true;
        PlayerSettings.SetUseDefaultGraphicsAPIs(BuildTarget.StandaloneWindows64, false);
        PlayerSettings.SetGraphicsAPIs(BuildTarget.StandaloneWindows64, new[] {
            GraphicsDeviceType.Direct3D11,
            GraphicsDeviceType.Direct3D12,
            GraphicsDeviceType.Vulkan,
            GraphicsDeviceType.OpenGLCore
        });
        PlayerSettings.SetScriptingBackend(NamedBuildTarget.Standalone, backend);
        AssetDatabase.SaveAssets();
    }

    static void CreateScene() {
        Directory.CreateDirectory("Assets/Scenes");
        Directory.CreateDirectory("Assets/Materials");
        var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);

        var cameraObject = new GameObject("Main Camera");
        cameraObject.tag = "MainCamera";
        var camera = cameraObject.AddComponent<Camera>();
        camera.clearFlags = CameraClearFlags.SolidColor;
        camera.backgroundColor = new Color(0.12f, 0.2f, 0.35f);
        cameraObject.transform.position = new Vector3(0f, 0f, -5f);

        var lightObject = new GameObject("Light");
        var light = lightObject.AddComponent<Light>();
        light.type = LightType.Directional;
        lightObject.transform.rotation = Quaternion.Euler(50f, -30f, 0f);

        var cube = GameObject.CreatePrimitive(PrimitiveType.Cube);
        cube.name = "Cube";
        var material = new Material(Shader.Find("Standard")) { color = new Color(0.9f, 0.45f, 0.1f) };
        AssetDatabase.CreateAsset(material, "Assets/Materials/Cube.mat");
        cube.GetComponent<MeshRenderer>().sharedMaterial = material;

        var canvasObject = new GameObject("Canvas");
        var canvas = canvasObject.AddComponent<Canvas>();
        canvas.renderMode = RenderMode.ScreenSpaceOverlay;
        canvasObject.AddComponent<CanvasScaler>();
        canvasObject.AddComponent<GraphicRaycaster>();

        var labelObject = new GameObject("Label");
        labelObject.transform.SetParent(canvasObject.transform, false);
        var text = labelObject.AddComponent<Text>();
        text.font = Resources.GetBuiltinResource<Font>("LegacyRuntime.ttf");
        text.fontSize = 36;
        text.color = Color.white;
        text.alignment = TextAnchor.UpperRight;
        text.text = "uGUI";
        var rect = text.rectTransform;
        rect.anchorMin = new Vector2(1f, 1f);
        rect.anchorMax = new Vector2(1f, 1f);
        rect.pivot = new Vector2(1f, 1f);
        rect.anchoredPosition = new Vector2(-20f, -20f);
        rect.sizeDelta = new Vector2(700f, 60f);

        var game = new GameObject("TestGame").AddComponent<TestGame>();
        game.cube = cube.transform;
        game.label = text;

        EditorSceneManager.SaveScene(scene, ScenePath);
        AssetDatabase.SaveAssets();
    }
}
