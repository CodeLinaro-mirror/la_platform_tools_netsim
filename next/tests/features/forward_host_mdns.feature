@skip
Feature: Host mDNS Forwarding

Scenario: Guest discovers service advertised by host
    When @host advertises mDNS service _test._tcp.
    Then @avd:1 finds service _test._tcp.
