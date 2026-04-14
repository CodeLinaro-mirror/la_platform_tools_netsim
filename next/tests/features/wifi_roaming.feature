Feature: Wi-Fi Roaming Connection

Scenario: Roam between Open and Secured AP
  Given @adb has 1 attached device
  And @adb:1 disables cellular data
  When @adb:1 connects to open wifi "AndroidWifi"
  And @host starts a TCP echo server on "port_1"
  Then @android:1 sends 2KB TCP to {port_1}
  And @host receives all coordinated data

  When @adb:1 connects to wifi "Coffee" with password "12345678"
  And @host starts a TCP echo server on "port_2"
  Then @android:1 sends 2KB TCP to {port_2}
  And @host receives all coordinated data

  # Roam Back! This verifies the multi-AP BSSID tracking fix.
  When @adb:1 connects to open wifi "AndroidWifi"
  And @host starts a TCP echo server on "port_3"
  Then @android:1 sends 2KB TCP to {port_3}
  And @host receives all coordinated data
