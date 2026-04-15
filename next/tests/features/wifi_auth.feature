@skip
Feature: Wi-Fi Authentication

Scenario: Connect to Secured AP
  Given @adb has 1 attached device
  And @adb:1 disables cellular data
  When @adb:1 connects to wifi "AndroidWifi" with password "12345678"
  And @host starts a TCP echo server on "port"
  Then @android:1 sends 2KB TCP to {port}
  And @host receives all coordinated data
