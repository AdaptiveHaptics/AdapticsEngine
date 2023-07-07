#include <exception>

#include "lmc-track.h"

#include "adaptics-engine/src/threads/tracking/leapmotion/ffi.rs.h"

/** Translates eLeapRS result codes into a human-readable string. */
const char* ResultString(eLeapRS r) {
  switch(r){
    case eLeapRS_Success:                  return "eLeapRS_Success";
    case eLeapRS_UnknownError:             return "eLeapRS_UnknownError";
    case eLeapRS_InvalidArgument:          return "eLeapRS_InvalidArgument";
    case eLeapRS_InsufficientResources:    return "eLeapRS_InsufficientResources";
    case eLeapRS_InsufficientBuffer:       return "eLeapRS_InsufficientBuffer";
    case eLeapRS_Timeout:                  return "eLeapRS_Timeout";
    case eLeapRS_NotConnected:             return "eLeapRS_NotConnected";
    case eLeapRS_HandshakeIncomplete:      return "eLeapRS_HandshakeIncomplete";
    case eLeapRS_BufferSizeOverflow:       return "eLeapRS_BufferSizeOverflow";
    case eLeapRS_ProtocolError:            return "eLeapRS_ProtocolError";
    case eLeapRS_InvalidClientID:          return "eLeapRS_InvalidClientID";
    case eLeapRS_UnexpectedClosed:         return "eLeapRS_UnexpectedClosed";
    case eLeapRS_UnknownImageFrameRequest: return "eLeapRS_UnknownImageFrameRequest";
    case eLeapRS_UnknownTrackingFrameID:   return "eLeapRS_UnknownTrackingFrameID";
    case eLeapRS_RoutineIsNotSeer:         return "eLeapRS_RoutineIsNotSeer";
    case eLeapRS_TimestampTooEarly:        return "eLeapRS_TimestampTooEarly";
    case eLeapRS_ConcurrentPoll:           return "eLeapRS_ConcurrentPoll";
    case eLeapRS_NotAvailable:             return "eLeapRS_NotAvailable";
    case eLeapRS_NotStreaming:             return "eLeapRS_NotStreaming";
    case eLeapRS_CannotOpenDevice:         return "eLeapRS_CannotOpenDevice";
    default:                               return "unknown result type.";
  }
}

#define throw_if_error(res) if (res != eLeapRS_Success) { throw std::exception(ResultString(res)); }
void unwrap(eLeapRS res) {
	throw_if_error(res);
}

// cb_func returns false to stop the message pump loop
void OpenConnectionAndStartMessagePump(rust::Fn<void(RawTrackingCoords const &)> cb_func, rust::Fn<bool()> is_done) {
	LEAP_CONNECTION connectionHandle;
	eLeapRS res;
	res = LeapCreateConnection(NULL, &connectionHandle);
	unwrap(res);
	res = LeapOpenConnection(connectionHandle);
	unwrap(res);


	LEAP_CONNECTION_MESSAGE msg;
	bool IsConnected;
	while(!is_done()) {
		unsigned int timeout = 1000;
        res = LeapPollConnection(connectionHandle, timeout, &msg);
		if (res != eLeapRS_Timeout) {
			unwrap(res);

			switch (msg.type){
				case eLeapEventType_Connection:
					IsConnected = true;
					break;
				case eLeapEventType_ConnectionLost:
					IsConnected = false;
					break;
				case eLeapEventType_Device:
					// handleDeviceEvent(msg.device_event);
					break;
				case eLeapEventType_DeviceLost:
					// handleDeviceLostEvent(msg.device_event);
					break;
				case eLeapEventType_DeviceFailure:
					// handleDeviceFailureEvent(msg.device_failure_event);
					break;
				case eLeapEventType_Tracking: {
					RawTrackingCoords tracking_coords;
					const LEAP_TRACKING_EVENT* frame = msg.tracking_event;
					if (
						!frame || frame->tracking_frame_id <= 0 // There is no frame
						|| frame->nHands == 0 // There are no hands
					) {
						tracking_coords.has_hand = false;
					} else {
						LEAP_HAND *hand = &frame->pHands[0];
						// Get the palm position in the Leap coordinate system
						LEAP_VECTOR leap_palm_position = hand->palm.position;

						tracking_coords.has_hand = true;
						tracking_coords.x = leap_palm_position.x;
						tracking_coords.y = leap_palm_position.y;
						tracking_coords.z = leap_palm_position.z;
					}

					// cb_func(tracking_coords);
					break;
				}
				case eLeapEventType_ImageComplete:
					// Ignore
					break;
				case eLeapEventType_ImageRequestError:
					// Ignore
					break;
				case eLeapEventType_LogEvent:
					// handleLogEvent(msg.log_event);
					break;
				case eLeapEventType_Policy:
					// handlePolicyEvent(msg.policy_event);
					break;
				case eLeapEventType_ConfigChange:
					// handleConfigChangeEvent(msg.config_change_event);
					break;
				case eLeapEventType_ConfigResponse:
					// handleConfigResponseEvent(msg.config_response_event);
					break;
				case eLeapEventType_Image:
					// handleImageEvent(msg.image_event);
					break;
				case eLeapEventType_PointMappingChange:
					// handlePointMappingChangeEvent(msg.point_mapping_change_event);
					break;
				case eLeapEventType_LogEvents:
					// handleLogEvents(msg.log_events);
					break;
				case eLeapEventType_HeadPose:
					// handleHeadPoseEvent(msg.head_pose_event);
					break;
				default:
					//discard unknown message types
					printf("Unhandled message type %i.\n", msg.type);
			}
		}
    }

	LeapCloseConnection(connectionHandle);
	LeapDestroyConnection(connectionHandle);
}