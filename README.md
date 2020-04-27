# ArcamClient

This gtk-rs based Rust application is to control an Arcam AVR amplifier or AV receiver over an Ethernet
connection.

[![Build status](https://gitlab.com/Russel/arcamclient/badges/master/build.svg)](https://gitlab.com/Russel/arcamclient)
[![Licence](https://img.shields.io/badge/license-GPL_3-green.svg)](https://www.gnu.org/licenses/gpl-3.0.txt)

## Background

This work was started using an AVR600, but stalled when it appeared that my AVR600 was not behaving as the
manual stated it should.  The project went into hiatus when Arcam admitted the system of controlling the
amplifier over a TCP connection on an AVR600 did not work. When my AVR600 broke (again), rather than get it
repaired (again), I decided to replace it with an AVR850. Since the TCP connection based control system
works on this amplifier, work on this application has restarted.

## AVR and Ethernet

The AVR amplifiers and receivers have Ethernet connectivity in order to access online media resources.  The
Arcam folk also put a Web server into the AVR600 to provide status information, but this appears not to be
present in the AVR850.

On the AVR600 port 50001 was a TCP server socket that allows the IR controller / RS232 controller protocols to
be used over Ethernet. On the AVR850 the port has been changed to 50000.

## Licence

This code is licenced under GPLv3. [![Licence](https://img.shields.io/badge/license-GPL_3-green.svg)](https://www.gnu.org/licenses/gpl-3.0.txt)
