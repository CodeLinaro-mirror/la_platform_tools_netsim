
Feature: Service Discovery

Scenario: P2P Discovery
    Given @avd has 2 attached devices
    When @avd:1 advertises service "MyService" as "_test._tcp"
    Then @avd:2 finds service "_test._tcp"
