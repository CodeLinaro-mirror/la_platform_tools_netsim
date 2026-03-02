Feature: WiFi Service Discovery
  Scenario: mDNS Loopback
    Given @adb has 1 attached device
    When @android:1 advertises service _http._tcp. as MyLoopbackService
    And @android:1 starts discovery for _http._tcp.
    Then @android:1 finds service MyLoopbackService

  Scenario: mDNS/NSD Cross-Device
    Given @adb has 2 attached devices
    When @android:1 advertises service _http._tcp. as MyWiFiService
    And @android:2 starts discovery for _http._tcp.
    Then @android:2 finds service MyWiFiService
