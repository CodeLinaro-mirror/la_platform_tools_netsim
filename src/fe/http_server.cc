// Copyright 2022 The Android Open Source Project
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#include "fe/http_server.h"

#include <google/protobuf/empty.pb.h>
#include <google/protobuf/util/json_util.h>
#include <libwebsockets.h>

#include <string>

#include "controller/device_notify_manager.h"
#include "controller/scene_controller.h"
#include "frontend.pb.h"

namespace netsim::http {

namespace {

const unsigned char kCorsHeaderKey[] = "Access-Control-Allow-Origin:";
const unsigned char kCorsHeaderValue[] = "*";

class Status {
 public:
  Status() = default;
  Status(bool ok) : ok_(ok) { code_ = HTTP_STATUS_OK; }
  Status(unsigned int code, std::string error_message)
      : code_(code), error_message_(std::move(error_message)) {
    ok_ = false;
  }
  bool Ok() { return ok_; };
  unsigned int Code() { return code_; };

  std::string GetJsonString() {
    return "{\"code\":" + std::to_string(code_) +
           ", \"error_message\":" + error_message_ + "}";
  }

 private:
  bool ok_;
  unsigned int code_;
  std::string error_message_;
};

std::string GetEnv(const std::string &name, const std::string &default_value) {
  auto val = std::getenv(name.c_str());
  if (!val) {
    return default_value;
  }
  return val;
}

Status GetVersion(const std::string &request, std::string &response) {
  frontend::VersionResponse response_proto;
  response_proto.set_version("123b");
  google::protobuf::util::MessageToJsonString(response_proto, &response);
  return Status(true);
}

Status SetPosition(const std::string &request, std::string &response) {
  frontend::SetPositionRequest request_proto;
  google::protobuf::util::JsonStringToMessage(request, &request_proto);
  google::protobuf::Empty response_proto;

  auto status = netsim::controller::SceneController::Singleton().SetPosition(
      request_proto.device_serial(), request_proto.position());
  if (!status) {
    return Status(HTTP_STATUS_BAD_REQUEST,
                  "device_serial not found: " + request_proto.device_serial());
  }

  google::protobuf::util::MessageToJsonString(response_proto, &response);
  return Status(true);
}

Status UpdateDevice(const std::string &request, std::string &response) {
  frontend::UpdateDeviceRequest request_proto;
  google::protobuf::util::JsonStringToMessage(request, &request_proto);
  google::protobuf::Empty response_proto;

  auto status = netsim::controller::SceneController::Singleton().UpdateDevice(
      request_proto.device());
  if (!status) {
    return Status(
        HTTP_STATUS_BAD_REQUEST,
        "device_serial not found: " + request_proto.device().device_serial());
  }

  google::protobuf::util::MessageToJsonString(response_proto, &response);
  return Status(true);
}

Status GetDevices(const std::string &request, std::string &response) {
  frontend::GetDevicesResponse response_proto;
  const auto &devices = netsim::controller::SceneController::Singleton().Copy();
  for (const auto &device : devices)
    response_proto.add_devices()->CopyFrom(device->model);

  google::protobuf::util::MessageToJsonString(response_proto, &response);
  return Status(true);
}

Status Reset(const std::string &request, std::string &response) {
  netsim::controller::SceneController::Singleton().Reset();
  google::protobuf::Empty response_proto;
  google::protobuf::util::MessageToJsonString(response_proto, &response);
  return Status(true);
}

Status SetPacketCapture(const std::string &request, std::string &response) {
  frontend::SetPacketCaptureRequest request_proto;
  google::protobuf::util::JsonStringToMessage(request, &request_proto);
  google::protobuf::Empty response_proto;

  model::Device device;
  device.set_device_serial(request_proto.device_serial());
  model::Chip chip;
  // Turn on bt packet capture
  chip.set_capture(request_proto.capture() ? model::State::ON
                                           : model::State::OFF);
  chip.mutable_bt();
  device.mutable_chips()->Add()->CopyFrom(chip);
  controller::SceneController::Singleton().UpdateDevice(device);

  google::protobuf::util::MessageToJsonString(response_proto, &response);
  return Status(true);
}

// Per-session user data.
struct Session {
  std::string path;
  bool called_scene_controller = false;
  bool sent_header = false;
  bool sent_body = false;
  Status status;
  std::string request_body = "";
  std::string response = "";
  // Optional for register-updates request.
  unsigned int registered_callback_id = 0;
};

int callback_http(struct lws *wsi, enum lws_callback_reasons reason, void *user,
                  void *in, size_t len) {
  auto *session = reinterpret_cast<Session *>(user);
  uint8_t buf[LWS_PRE + LWS_RECOMMENDED_MIN_HEADER_SPACE],
      *start = &buf[LWS_PRE], *p = start,
      *end = &buf[sizeof(buf) - LWS_PRE - 1];

  switch (reason) {
    case LWS_CALLBACK_HTTP:
      session->path = std::string(reinterpret_cast<const char *>(in));

      lws_get_peer_simple(wsi, reinterpret_cast<char *>(buf), sizeof(buf));
      lwsl_notice("HTTP: connection %s, path %s\n",
                  reinterpret_cast<const char *>(buf), session->path.c_str());

      if (session->path.compare("/register-updates") == 0) {
        session->registered_callback_id =
            controller::DeviceNotifyManager::Get().Register(
                [wsi]() -> void { lws_callback_on_writable(wsi); });
        return 0;
      }

      if (lws_hdr_total_length(wsi, WSI_TOKEN_GET_URI))  // GET
        // write the body separately
        lws_callback_on_writable(wsi);
      return 0;

    case LWS_CALLBACK_HTTP_BODY:
      session->request_body =
          std::string(reinterpret_cast<const char *>(in), len);
      return 0;

    case LWS_CALLBACK_HTTP_BODY_COMPLETION:
      lws_callback_on_writable(wsi);
      return 0;

    case LWS_CALLBACK_HTTP_WRITEABLE:
      if (!session) break;

      if (!session->called_scene_controller) {
        if (session->path.compare("/get-version") == 0) {
          session->status =
              GetVersion(session->request_body, session->response);
        } else if (session->path.compare("/get-devices") == 0) {
          session->status =
              GetDevices(session->request_body, session->response);
        } else if (session->path.compare("/set-position") == 0) {
          session->status =
              SetPosition(session->request_body, session->response);
        } else if (session->path.compare("/reset") == 0) {
          session->status = Reset(session->request_body, session->response);
        } else if (session->path.compare("/update-device") == 0) {
          session->status =
              UpdateDevice(session->request_body, session->response);
        } else if (session->path.compare("/set-packet-capture") == 0) {
          session->status =
              SetPacketCapture(session->request_body, session->response);
        } else if (session->path.compare("/register-updates") == 0) {
          // Wake up by DeviceNotifyManager.
          session->status =
              GetDevices(session->request_body, session->response);
        } else {
          session->status = Status(HTTP_STATUS_NOT_FOUND, "invalid url");
        }

        if (!session->status.Ok())
          session->response = session->status.GetJsonString();
        session->called_scene_controller = true;
      }

      if (!session->sent_header) {
        if (lws_add_http_common_headers(
                wsi, session->status.Code(), "application/json",
                LWS_ILLEGAL_HTTP_CONTENT_LEN, /* no content len */
                &p, end))
          return 1;

        if (lws_add_http_header_by_name(
                wsi, kCorsHeaderKey, kCorsHeaderValue,
                sizeof(kCorsHeaderValue) - 1,  // Exclude the null character.
                &p, end))
          return 1;
        if (lws_finalize_write_http_header(wsi, start, &p, end)) return 1;

        lws_callback_on_writable(wsi);
        session->sent_header = true;
        return 0;
      }

      if (!session->sent_body) {
        lws_write(wsi, (uint8_t *)session->response.c_str(),
                  session->response.size(), LWS_WRITE_HTTP_FINAL);
        session->sent_body = true;
      }

      if (lws_http_transaction_completed(wsi)) return -1;

      return 0;

    case LWS_CALLBACK_CLOSED_HTTP:
      if (session->registered_callback_id > 0)
        controller::DeviceNotifyManager::Get().Unregister(
            session->registered_callback_id);

      return 0;

    default:
      break;
  }

  return lws_callback_http_dummy(wsi, reason, user, in, len);
}

static const struct lws_protocols protocol = {
    "http", callback_http, sizeof(Session), 0, 0, NULL, 0};

static const struct lws_protocols *pprotocols[] = {&protocol, NULL};

// override the default mount for /netsim in the URL space

static const struct lws_http_mount mount_netsim = {
    /* .mount_next */ NULL,       // linked-list "next"
    /* .mountpoint */ "/netsim",  // mountpoint URL
    /* .origin */ NULL,           // protocol
    /* .def */ NULL,
    /* .protocol */ "http",
    /* .cgienv */ NULL,
    /* .extra_mimetypes */ NULL,
    /* .interpret */ NULL,
    /* .cgi_timeout */ 0,
    /* .cache_max_age */ 0,
    /* .auth_mask */ 0,
    /* .cache_reusable */ 0,
    /* .cache_revalidate */ 0,
    /* .cache_intermediaries */ 0,
    /* .origin_protocol */ LWSMPRO_CALLBACK,  // dynamic
    /* .mountpoint_len */ 7,                  // char count of "/netsim"
    /* .basic_auth_login_file */ NULL,
};
auto origin = GetEnv("HOME", ".") + "/netsim-web";
static const struct lws_http_mount mount = {
    /* .mount_next */ &mount_netsim,  // linked-list "next"
    /* .mountpoint */ "/",            // mountpoint URL
    /* .origin */ origin.c_str(),     // serve from dir
    /* .def */ "index.html",          // default filename
    /* .protocol */ NULL,
    /* .cgienv */ NULL,
    /* .extra_mimetypes */ NULL,
    /* .interpret */ NULL,
    /* .cgi_timeout */ 0,
    /* .cache_max_age */ 0,
    /* .auth_mask */ 0,
    /* .cache_reusable */ 0,
    /* .cache_revalidate */ 0,
    /* .cache_intermediaries */ 0,
    /* .origin_protocol */ LWSMPRO_FILE,  // files in a dir
    /* .mountpoint_len */ 1,              // char count
    /* .basic_auth_login_file */ NULL,
};

}  // namespace

void RunHttpServer() {
  lws_context_creation_info info;
  lws_context *context;
  const char *p;
  int n = 0, logs = LLL_USER | LLL_ERR | LLL_WARN | LLL_NOTICE;

  lws_set_log_level(logs, NULL);
  lwsl_user("netsim http server is listening on http://localhost:7681\n");
  lwsl_user("netsim https server is listening on https://localhost:7682\n");

  memset(&info, 0, sizeof info);  // otherwise uninitialized garbage
  info.options = LWS_SERVER_OPTION_DO_SSL_GLOBAL_INIT |
                 LWS_SERVER_OPTION_EXPLICIT_VHOSTS |
                 LWS_SERVER_OPTION_HTTP_HEADERS_SECURITY_BEST_PRACTICES_ENFORCE;
  // NOTE: Web server terminates after 15 seconds and cause Http request
  // timeout.

  context = lws_create_context(&info);
  if (!context) {
    lwsl_err("lws init failed\n");
    return;
  }

  std::string certs_dir = GetEnv("HOME", ".") + "/usr/share/webrtc/certs/";
  std::string cert_file = certs_dir + "server.crt";
  std::string key_file = certs_dir + "server.key";

  info.iface = "127.0.0.1";  // listen only on localhost
  // Run HTTP service on 7681.
  info.port = 7681;
  info.pprotocols = pprotocols;
  info.mounts = &mount;
  info.vhost_name = "http";

  if (!lws_create_vhost(context, &info)) {
    lwsl_err("Failed to create tls vhost\n");
    goto bail;
  }

  // Run HTTPS service on 7682.
  info.port = 7682;
#if defined(LWS_WITH_TLS)
  info.ssl_cert_filepath = cert_file.c_str();
  info.ssl_private_key_filepath = key_file.c_str();
#endif
  info.vhost_name = "https";

  if (!lws_create_vhost(context, &info)) {
    lwsl_err("Failed to create tls vhost\n");
    goto bail;
  }

  while (n >= 0) n = lws_service(context, 0);

bail:
  lws_context_destroy(context);

  return;
}

}  // namespace netsim::http
