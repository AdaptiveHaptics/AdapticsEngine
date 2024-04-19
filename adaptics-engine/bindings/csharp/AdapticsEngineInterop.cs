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
            if (api_version != 12559145612103383281ul)
            {
                throw new TypeLoadException($"API reports hash {api_version} which differs from hash in bindings (12559145612103383281). You probably forgot to update / copy either the bindings or the library.");
            }
        }


        /// Initializes the Adaptics Engine, returns a handle ID.
        ///
        /// use_mock_streaming: if true, use mock streaming. if false, use ulhaptics streaming.
        ///
        /// enable_playback_updates: if true, enable playback updates, adaptics_engine_get_playback_updates expected to be called at (1/SECONDS_PER_PLAYBACK_UPDATE)hz.
        ///
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "init_adaptics_engine")]
        public static extern ulong init_adaptics_engine(bool use_mock_streaming, bool enable_playback_updates);

        /// Deinitializes the Adaptics Engine.
        /// Returns with an error message if available.
        ///
        /// The unity package uses a err_msg buffer of size 1024.
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "deinit_adaptics_engine")]
        public static extern FFIError deinit_adaptics_engine(ulong handle_id, SliceMutu8 err_msg);

        /// Deinitializes the Adaptics Engine.
        /// Returns with an error message if available.
        ///
        /// The unity package uses a err_msg buffer of size 1024.
        public static void deinit_adaptics_engine(ulong handle_id, byte[] err_msg)
        {
            var err_msg_pinned = GCHandle.Alloc(err_msg, GCHandleType.Pinned);
            var err_msg_slice = new SliceMutu8(err_msg_pinned, (ulong) err_msg.Length);
            try
            {
                var rval = deinit_adaptics_engine(handle_id, err_msg_slice);;
                if (rval != FFIError.Ok)
                {
                    throw new InteropException<FFIError>(rval);
                }
            }
            finally
            {
                err_msg_pinned.Free();
            }
        }

        /// Updates the pattern to be played.
        /// For further information, see [PatternEvalUpdate::Pattern].
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "adaptics_engine_update_pattern")]
        public static extern FFIError adaptics_engine_update_pattern(ulong handle_id, string pattern_json);

        /// Updates the pattern to be played.
        /// For further information, see [PatternEvalUpdate::Pattern].
        public static void adaptics_engine_update_pattern_checked(ulong handle_id, string pattern_json)
        {
            var rval = adaptics_engine_update_pattern(handle_id, pattern_json);;
            if (rval != FFIError.Ok)
            {
                throw new InteropException<FFIError>(rval);
            }
        }

        /// Alias for [crate::adaptics_engine_update_pattern()]
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "adaptics_engine_update_tacton")]
        public static extern FFIError adaptics_engine_update_tacton(ulong handle_id, string pattern_json);

        /// Alias for [crate::adaptics_engine_update_pattern()]
        public static void adaptics_engine_update_tacton_checked(ulong handle_id, string pattern_json)
        {
            var rval = adaptics_engine_update_tacton(handle_id, pattern_json);;
            if (rval != FFIError.Ok)
            {
                throw new InteropException<FFIError>(rval);
            }
        }

        /// Used to start and stop playback.
        /// For further information, see [PatternEvalUpdate::Playstart].
        ///
        /// To correctly start in the middle of a pattern, ensure that the time parameter is set appropriately before initiating playback.
        /// Use [adaptics_engine_update_time()] or [adaptics_engine_update_parameters()] to set the time parameter.
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "adaptics_engine_update_playstart")]
        public static extern FFIError adaptics_engine_update_playstart(ulong handle_id, double playstart, double playstart_offset);

        /// Used to start and stop playback.
        /// For further information, see [PatternEvalUpdate::Playstart].
        ///
        /// To correctly start in the middle of a pattern, ensure that the time parameter is set appropriately before initiating playback.
        /// Use [adaptics_engine_update_time()] or [adaptics_engine_update_parameters()] to set the time parameter.
        public static void adaptics_engine_update_playstart_checked(ulong handle_id, double playstart, double playstart_offset)
        {
            var rval = adaptics_engine_update_playstart(handle_id, playstart, playstart_offset);;
            if (rval != FFIError.Ok)
            {
                throw new InteropException<FFIError>(rval);
            }
        }

        /// Used to update all evaluator_params.
        ///
        /// Accepts a JSON string representing the evaluator parameters. See [PatternEvaluatorParameters].
        /// For further information, see [PatternEvalUpdate::Parameters].
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "adaptics_engine_update_parameters")]
        public static extern FFIError adaptics_engine_update_parameters(ulong handle_id, string evaluator_params);

        /// Used to update all evaluator_params.
        ///
        /// Accepts a JSON string representing the evaluator parameters. See [PatternEvaluatorParameters].
        /// For further information, see [PatternEvalUpdate::Parameters].
        public static void adaptics_engine_update_parameters_checked(ulong handle_id, string evaluator_params)
        {
            var rval = adaptics_engine_update_parameters(handle_id, evaluator_params);;
            if (rval != FFIError.Ok)
            {
                throw new InteropException<FFIError>(rval);
            }
        }

        /// Resets all evaluator parameters to their default values.
        /// For further information, see [PatternEvalUpdate::Parameters].
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "adaptics_engine_reset_parameters")]
        public static extern FFIError adaptics_engine_reset_parameters(ulong handle_id);

        /// Resets all evaluator parameters to their default values.
        /// For further information, see [PatternEvalUpdate::Parameters].
        public static void adaptics_engine_reset_parameters_checked(ulong handle_id)
        {
            var rval = adaptics_engine_reset_parameters(handle_id);;
            if (rval != FFIError.Ok)
            {
                throw new InteropException<FFIError>(rval);
            }
        }

        /// Updates `evaluator_params.time`.
        ///
        /// To correctly start in the middle of a pattern, ensure that the time parameter is set appropriately before initiating playback.
        ///
        /// # Notes
        /// - `evaluator_params.time` will be overwritten by the playstart time computation during playback.
        /// - Setting `evaluator_params.time` will not cause any pattern evaluation to occur (no playback updates).
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "adaptics_engine_update_time")]
        public static extern FFIError adaptics_engine_update_time(ulong handle_id, double time);

        /// Updates `evaluator_params.time`.
        ///
        /// To correctly start in the middle of a pattern, ensure that the time parameter is set appropriately before initiating playback.
        ///
        /// # Notes
        /// - `evaluator_params.time` will be overwritten by the playstart time computation during playback.
        /// - Setting `evaluator_params.time` will not cause any pattern evaluation to occur (no playback updates).
        public static void adaptics_engine_update_time_checked(ulong handle_id, double time)
        {
            var rval = adaptics_engine_update_time(handle_id, time);;
            if (rval != FFIError.Ok)
            {
                throw new InteropException<FFIError>(rval);
            }
        }

        /// Updates all user parameters.
        /// Accepts a JSON string of user parameters in the format `{ [key: string]: double }`.
        /// For further information, see [PatternEvalUpdate::UserParameters].
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "adaptics_engine_update_user_parameters")]
        public static extern FFIError adaptics_engine_update_user_parameters(ulong handle_id, string user_parameters);

        /// Updates all user parameters.
        /// Accepts a JSON string of user parameters in the format `{ [key: string]: double }`.
        /// For further information, see [PatternEvalUpdate::UserParameters].
        public static void adaptics_engine_update_user_parameters_checked(ulong handle_id, string user_parameters)
        {
            var rval = adaptics_engine_update_user_parameters(handle_id, user_parameters);;
            if (rval != FFIError.Ok)
            {
                throw new InteropException<FFIError>(rval);
            }
        }

        /// Updates a single user parameter.
        /// Accepts a JSON string of user parameters in the format `{ [key: string]: double }`.
        /// For further information, see [PatternEvalUpdate::UserParameters].
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "adaptics_engine_update_user_parameter")]
        public static extern FFIError adaptics_engine_update_user_parameter(ulong handle_id, string name, double value);

        /// Updates a single user parameter.
        /// Accepts a JSON string of user parameters in the format `{ [key: string]: double }`.
        /// For further information, see [PatternEvalUpdate::UserParameters].
        public static void adaptics_engine_update_user_parameter_checked(ulong handle_id, string name, double value)
        {
            var rval = adaptics_engine_update_user_parameter(handle_id, name, value);;
            if (rval != FFIError.Ok)
            {
                throw new InteropException<FFIError>(rval);
            }
        }

        /// Updates `geo_matrix`, a 4x4 matrix in row-major order, where `data[3]` is the fourth element of the first row (translate x).
        /// For further information, see [PatternEvalUpdate::GeoTransformMatrix].
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "adaptics_engine_update_geo_transform_matrix")]
        public static extern FFIError adaptics_engine_update_geo_transform_matrix(ulong handle_id, GeoMatrix geo_matrix);

        /// Updates `geo_matrix`, a 4x4 matrix in row-major order, where `data[3]` is the fourth element of the first row (translate x).
        /// For further information, see [PatternEvalUpdate::GeoTransformMatrix].
        public static void adaptics_engine_update_geo_transform_matrix_checked(ulong handle_id, GeoMatrix geo_matrix)
        {
            var rval = adaptics_engine_update_geo_transform_matrix(handle_id, geo_matrix);;
            if (rval != FFIError.Ok)
            {
                throw new InteropException<FFIError>(rval);
            }
        }

        /// Populate `eval_results` with the latest evaluation results.
        /// `num_evals` will be set to the number of evaluations written to `eval_results`, or 0 if there are no new evaluations since the last call to this function.
        ///
        /// # Safety
        /// `num_evals` must be a valid pointer to a u32
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "adaptics_engine_get_playback_updates")]
        public static extern FFIError adaptics_engine_get_playback_updates(ulong handle_id, ref SliceMutUnityEvalResult eval_results, out uint num_evals);

        /// Populate `eval_results` with the latest evaluation results.
        /// `num_evals` will be set to the number of evaluations written to `eval_results`, or 0 if there are no new evaluations since the last call to this function.
        ///
        /// # Safety
        /// `num_evals` must be a valid pointer to a u32
        public static void adaptics_engine_get_playback_updates(ulong handle_id, UnityEvalResult[] eval_results, out uint num_evals)
        {
            var eval_results_pinned = GCHandle.Alloc(eval_results, GCHandleType.Pinned);
            var eval_results_slice = new SliceMutUnityEvalResult(eval_results_pinned, (ulong) eval_results.Length);
            try
            {
                var rval = adaptics_engine_get_playback_updates(handle_id, ref eval_results_slice, out num_evals);;
                if (rval != FFIError.Ok)
                {
                    throw new InteropException<FFIError>(rval);
                }
            }
            finally
            {
                eval_results_pinned.Free();
            }
        }

        /// Higher level function to load a new pattern and instantly start playback.
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "adaptics_engine_play_tacton_immediate")]
        public static extern FFIError adaptics_engine_play_tacton_immediate(ulong handle_id, string tacton_json);

        /// Higher level function to load a new pattern and instantly start playback.
        public static void adaptics_engine_play_tacton_immediate_checked(ulong handle_id, string tacton_json)
        {
            var rval = adaptics_engine_play_tacton_immediate(handle_id, tacton_json);;
            if (rval != FFIError.Ok)
            {
                throw new InteropException<FFIError>(rval);
            }
        }

        /// Guard function used by bindings.
        ///
        /// Change impl version in this comment to force bump the API version.
        /// impl_version: 1
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "ffi_api_guard")]
        public static extern ulong ffi_api_guard();

    }

    /// Defines a 4x4 matrix in row-major order for FFI.
    [Serializable]
    [StructLayout(LayoutKind.Sequential)]
    public partial struct GeoMatrix
    {
        public double data0;
        public double data1;
        public double data2;
        public double data3;
        public double data4;
        public double data5;
        public double data6;
        public double data7;
        public double data8;
        public double data9;
        public double data10;
        public double data11;
        public double data12;
        public double data13;
        public double data14;
        public double data15;
    }

    /// !NOTE: y and z are swapped for Unity
    [Serializable]
    [StructLayout(LayoutKind.Sequential)]
    public partial struct UnityEvalCoords
    {
        public double x;
        public double y;
        public double z;
    }

    /// !NOTE: y and z are swapped for Unity
    [Serializable]
    [StructLayout(LayoutKind.Sequential)]
    public partial struct UnityEvalResult
    {
        /// !NOTE: y and z are swapped for Unity
        public UnityEvalCoords coords;
        public double intensity;
        public double pattern_time;
        [MarshalAs(UnmanagedType.I1)]
        public bool stop;
    }

    public enum FFIError
    {
        Ok = 0,
        NullPassed = 1,
        Panic = 2,
        OtherError = 3,
        AdapticsEngineThreadDisconnectedCheckDeinitForMoreInfo = 4,
        ErrMsgProvided = 5,
        EnablePlaybackUpdatesWasFalse = 6,
        ParameterJSONDeserializationFailed = 8,
        HandleIDNotFound = 9,
    }

    ///A pointer to an array of data someone else owns which may be modified.
    [Serializable]
    [StructLayout(LayoutKind.Sequential)]
    public partial struct SliceMutUnityEvalResult
    {
        ///Pointer to start of mutable data.
        IntPtr data;
        ///Number of elements.
        ulong len;
    }

    public partial struct SliceMutUnityEvalResult : IEnumerable<UnityEvalResult>
    {
        public SliceMutUnityEvalResult(GCHandle handle, ulong count)
        {
            this.data = handle.AddrOfPinnedObject();
            this.len = count;
        }
        public SliceMutUnityEvalResult(IntPtr handle, ulong count)
        {
            this.data = handle;
            this.len = count;
        }
        public UnityEvalResult this[int i]
        {
            get
            {
                if (i >= Count) throw new IndexOutOfRangeException();
                var size = Marshal.SizeOf(typeof(UnityEvalResult));
                var ptr = new IntPtr(data.ToInt64() + i * size);
                return Marshal.PtrToStructure<UnityEvalResult>(ptr);
            }
            set
            {
                if (i >= Count) throw new IndexOutOfRangeException();
                var size = Marshal.SizeOf(typeof(UnityEvalResult));
                var ptr = new IntPtr(data.ToInt64() + i * size);
                Marshal.StructureToPtr<UnityEvalResult>(value, ptr, false);
            }
        }
        public UnityEvalResult[] Copied
        {
            get
            {
                var rval = new UnityEvalResult[len];
                for (var i = 0; i < (int) len; i++) {
                    rval[i] = this[i];
                }
                return rval;
            }
        }
        public int Count => (int) len;
        public IEnumerator<UnityEvalResult> GetEnumerator()
        {
            for (var i = 0; i < (int)len; ++i)
            {
                yield return this[i];
            }
        }
        IEnumerator IEnumerable.GetEnumerator()
        {
            return this.GetEnumerator();
        }
    }


    ///A pointer to an array of data someone else owns which may be modified.
    [Serializable]
    [StructLayout(LayoutKind.Sequential)]
    public partial struct SliceMutu8
    {
        ///Pointer to start of mutable data.
        IntPtr data;
        ///Number of elements.
        ulong len;
    }

    public partial struct SliceMutu8 : IEnumerable<byte>
    {
        public SliceMutu8(GCHandle handle, ulong count)
        {
            this.data = handle.AddrOfPinnedObject();
            this.len = count;
        }
        public SliceMutu8(IntPtr handle, ulong count)
        {
            this.data = handle;
            this.len = count;
        }
        public byte this[int i]
        {
            get
            {
                if (i >= Count) throw new IndexOutOfRangeException();
                var size = Marshal.SizeOf(typeof(byte));
                var ptr = new IntPtr(data.ToInt64() + i * size);
                return Marshal.PtrToStructure<byte>(ptr);
            }
            set
            {
                if (i >= Count) throw new IndexOutOfRangeException();
                var size = Marshal.SizeOf(typeof(byte));
                var ptr = new IntPtr(data.ToInt64() + i * size);
                Marshal.StructureToPtr<byte>(value, ptr, false);
            }
        }
        public byte[] Copied
        {
            get
            {
                var rval = new byte[len];
                for (var i = 0; i < (int) len; i++) {
                    rval[i] = this[i];
                }
                return rval;
            }
        }
        public int Count => (int) len;
        public IEnumerator<byte> GetEnumerator()
        {
            for (var i = 0; i < (int)len; ++i)
            {
                yield return this[i];
            }
        }
        IEnumerator IEnumerable.GetEnumerator()
        {
            return this.GetEnumerator();
        }
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
