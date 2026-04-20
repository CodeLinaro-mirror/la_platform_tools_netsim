
Feature: Service Discovery

Scenario: P2P Discovery
    Given @avd has 2 attached devices
    When @avd:1 advertises service _test._tcp. as MyService
    And @avd:2 starts discovery for _test._tcp.
    Then @avd:2 finds service MyService
