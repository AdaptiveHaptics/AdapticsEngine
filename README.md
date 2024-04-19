# Adaptics Engine
Facilitates playback of adaptive mid-air ultrasound haptic sensations created in the [Adaptics Designer](https://github.com/AdaptiveHaptics/AdapticsDesigner).

### [Download Latest Release](https://github.com/AdaptiveHaptics/AdapticsEngine/releases/latest) | [Documentation](#documentation) | [Publication](https://github.com/AdaptiveHaptics/AdapticsDesigner?tab=readme-ov-file#publication)

## About
This repository comprises two main crates: [`adaptics-engine`](https://github.com/AdaptiveHaptics/AdapticsEngine/tree/main/adaptics-engine) and [`adaptics-pattern-evaluator`](https://github.com/AdaptiveHaptics/AdapticsEngine/tree/main/adaptics-pattern-evaluator).

`adaptics-engine` offers a Command Line Interface (CLI) that runs a websocket server for real-time pattern playback from the Designer, supplemented with a C-compatible API and C# bindings. It is designed to be hardware agnostic, but currently targets [Ultraleap haptics devices](https://www.ultraleap.com/datasheets/STRATOS_Explore_Development_Kit_datasheet.pdf).

`adaptics-pattern-evaluator` facilitates the low-level evaluation of Adaptics patterns, and is incorporated into both the `adaptics-engine` and the Designer (via [WASM](https://webassembly.org/)).

# Installation
Due to the lack of an installer, please ensure:
- Installation of [Ultraleap Haptics SDK](https://developer.ultrahaptics.com/).
- Installation of [Ultraleap Gemini Tracking SDK (LeapC)](https://developer.leapmotion.com/tracking-software-download/).
- Accessibility of both `LeapC.dll` and `UltraleapHaptics.dll`, either through the PATH environment variable or just copied adjacent to `adaptics-engine-cli.exe`.

Please note that releases are available only for Windows at this time.

# Documentation
To generate the documentation, run:
```bash
cargo doc --no-deps --open
```
Or take a look at the [c bindings](https://github.com/AdaptiveHaptics/AdapticsEngine/blob/main/adaptics-engine/bindings/c/adapticsengine.h). Sorry for the inconvenience, the documentation is not yet available online.

If there are any bugs or questions please feel free to make an issue on the GitHub or reach out via email (kevin.john‮&#64;‬asu&#46;edu).