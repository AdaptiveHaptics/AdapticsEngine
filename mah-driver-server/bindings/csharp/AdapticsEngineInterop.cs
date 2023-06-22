// Automatically generated by Interoptopus.

#pragma warning disable 0105
using System;
using System.Collections;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using com.github.AdaptiveHaptics;
#pragma warning restore 0105

namespace com.github.AdaptiveHaptics
{
    public static partial class AdapticsEngineInterop
    {
        public const string NativeLib = "adaptics_engine";

        static AdapticsEngineInterop()
        {
            var api_version = AdapticsEngineInterop.ffi_api_guard();
            if (api_version != 12172712117313209754ul)
            {
                throw new TypeLoadException($"API reports hash {api_version} which differs from hash in bindings (12172712117313209754). You probably forgot to update / copy either the bindings or the library.");
            }
        }


        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "init_adaptics_engine")]
        public static extern IntPtr init_adaptics_engine(bool use_mock_streaming);

        /// # Safety
        /// `handle` must be a valid pointer to an `AdapticsEngineHandleFFI` allocated by `init_adaptics_engine`
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "deinit_adaptics_engine")]
        public static extern void deinit_adaptics_engine(IntPtr handle);

        /// # Safety
        /// `handle` must be a valid pointer to an `AdapticsEngineHandle` allocated by `init_adaptics_engine`
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "adaptics_engine_update_pattern")]
        public static extern void adaptics_engine_update_pattern(IntPtr handle, string pattern_json);

        /// # Safety
        /// `handle` must be a valid pointer to an `AdapticsEngineHandle` allocated by `init_adaptics_engine`
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "adaptics_engine_update_playstart")]
        public static extern void adaptics_engine_update_playstart(IntPtr handle, double playstart, double playstart_offset);

        /// # Safety
        /// `handle` must be a valid pointer to an `AdapticsEngineHandle` allocated by `init_adaptics_engine`
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "adaptics_engine_update_parameters")]
        public static extern void adaptics_engine_update_parameters(IntPtr handle, string evaluator_params);

        /// Guard function used by backends.
        ///
        /// Change impl version in this comment to force bump the API version.
        /// impl_version: 1
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "ffi_api_guard")]
        public static extern ulong ffi_api_guard();

    }



    public class InteropException<T> : Exception
    {
        public T Error { get; private set; }

        public InteropException(T error): base($"Something went wrong: {error}")
        {
            Error = error;
        }
    }

}
