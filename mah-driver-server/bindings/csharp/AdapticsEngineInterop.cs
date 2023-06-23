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
            if (api_version != 356762228646146003ul)
            {
                throw new TypeLoadException($"API reports hash {api_version} which differs from hash in bindings (356762228646146003). You probably forgot to update / copy either the bindings or the library.");
            }
        }


        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "init_adaptics_engine")]
        public static extern IntPtr init_adaptics_engine(bool use_mock_streaming);

        /// # Safety
        /// `handle` must be a valid pointer to an `AdapticsEngineHandleFFI` allocated by `init_adaptics_engine`
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "deinit_adaptics_engine")]
        public static extern FFIError deinit_adaptics_engine(IntPtr handle, SliceMutu8 err_msg);

        /// # Safety
        /// `handle` must be a valid pointer to an `AdapticsEngineHandleFFI` allocated by `init_adaptics_engine`
        public static void deinit_adaptics_engine(IntPtr handle, byte[] err_msg)
        {
            var err_msg_pinned = GCHandle.Alloc(err_msg, GCHandleType.Pinned);
            var err_msg_slice = new SliceMutu8(err_msg_pinned, (ulong) err_msg.Length);
            try
            {
                var rval = deinit_adaptics_engine(handle, err_msg_slice);;
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

        /// # Safety
        /// `handle` must be a valid pointer to an `AdapticsEngineHandle` allocated by `init_adaptics_engine`
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "adaptics_engine_update_pattern")]
        public static extern FFIError adaptics_engine_update_pattern(IntPtr handle, string pattern_json);

        /// # Safety
        /// `handle` must be a valid pointer to an `AdapticsEngineHandle` allocated by `init_adaptics_engine`
        public static void adaptics_engine_update_pattern_checked(IntPtr handle, string pattern_json)
        {
            var rval = adaptics_engine_update_pattern(handle, pattern_json);;
            if (rval != FFIError.Ok)
            {
                throw new InteropException<FFIError>(rval);
            }
        }

        /// # Safety
        /// `handle` must be a valid pointer to an `AdapticsEngineHandle` allocated by `init_adaptics_engine`
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "adaptics_engine_update_playstart")]
        public static extern FFIError adaptics_engine_update_playstart(IntPtr handle, double playstart, double playstart_offset);

        /// # Safety
        /// `handle` must be a valid pointer to an `AdapticsEngineHandle` allocated by `init_adaptics_engine`
        public static void adaptics_engine_update_playstart_checked(IntPtr handle, double playstart, double playstart_offset)
        {
            var rval = adaptics_engine_update_playstart(handle, playstart, playstart_offset);;
            if (rval != FFIError.Ok)
            {
                throw new InteropException<FFIError>(rval);
            }
        }

        /// # Safety
        /// `handle` must be a valid pointer to an `AdapticsEngineHandle` allocated by `init_adaptics_engine`
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "adaptics_engine_update_parameters")]
        public static extern FFIError adaptics_engine_update_parameters(IntPtr handle, string evaluator_params);

        /// # Safety
        /// `handle` must be a valid pointer to an `AdapticsEngineHandle` allocated by `init_adaptics_engine`
        public static void adaptics_engine_update_parameters_checked(IntPtr handle, string evaluator_params)
        {
            var rval = adaptics_engine_update_parameters(handle, evaluator_params);;
            if (rval != FFIError.Ok)
            {
                throw new InteropException<FFIError>(rval);
            }
        }

        /// Guard function used by backends.
        ///
        /// Change impl version in this comment to force bump the API version.
        /// impl_version: 1
        [DllImport(NativeLib, CallingConvention = CallingConvention.Cdecl, EntryPoint = "ffi_api_guard")]
        public static extern ulong ffi_api_guard();

    }

    public enum FFIError
    {
        Ok = 0,
        NullPassed = 1,
        Panic = 2,
        OtherError = 3,
        AdapticsEngineThreadDisconnectedCheckDeinitForMoreInfo = 4,
        ErrMsgProvided = 5,
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
