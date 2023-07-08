# Adaptics Engine

Adaptics Engine repository comprises two packages: `adaptics-engine` and `adaptics-pattern-evaluator`.

`adaptics-engine` offers a Command Line Interface (CLI) that runs a websocket server for real-time pattern playback from the Designer, supplemented with a C-compatible API and C# bindings. It is designed to be hardware agnostic, but currently targets [Ultraleap haptics devices](https://www.ultraleap.com/datasheets/STRATOS_Explore_Development_Kit_datasheet.pdf).

`adaptics-pattern-evaluator` facilitates the low-level evaluation of Adaptics patterns, and is incorporated into both the `adaptics-engine` and the Designer.

# Installation
Due to lack of an installer, please ensure:
- Installation of [Ultraleap Haptics SDK](https://developer.ultrahaptics.com/).
- Installation of [Ultraleap Gemini Tracking SDK (LeapC)](https://developer.leapmotion.com/tracking-software-download/).
- Accessibility of both `LeapC.dll` and `UltraleapHaptics.dll`, either through the PATH environment variable or just copied adjacent to `adaptics-engine-cli.exe`.

Please note that releases are available only for Windows at this time.

### [Download Latest Release](https://github.com/AdaptiveHaptics/AdapticsEngine/releases/latest)
