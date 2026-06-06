#!/usr/bin/env python3
"""
EdgeRun Bluetooth SSH Bridge
Protocol: bluetooth (spp-server + ble-gatt-server)
Builds: wasm32-wasi via wat2wasm, native x86_64 via edgerun compiler

Provides:
  1. RFCOMM SPP server for classic Bluetooth SSH
  2. BLE GATT UART service for Web Bluetooth
  3. Bridges both to SSH via localhost:22
"""

import sys
import os
import socket
import select
import threading
import subprocess
import signal
import struct
import time
import json
from typing import Optional

# ---- Configuration ----
SPP_CHANNEL = 1
BLE_SERVICE_NAME = "EdgeRun BT SSH"
NUS_SERVICE_UUID = "6E400001-B5A3-F393-E0A9-E50E24DCCA9E"
NUS_TX_UUID = "6E400002-B5A3-F393-E0A9-E50E24DCCA9E"
NUS_RX_UUID = "6E400003-B5A3-F393-E0A9-E50E24DCCA9E"
SSH_HOST = "127.0.0.1"
SSH_PORT = 22

running = True


# ---- SSH Bridge ----
def ssh_bridge(rfcomm_sock):
    """Bridge an RFCOMM socket to SSH via subprocess."""
    try:
        proc = subprocess.Popen(
            ["/usr/bin/ssh", f"-p{SSH_PORT}", f"{os.environ.get('USER', 'ken')}@{SSH_HOST}"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
    except FileNotFoundError:
        try:
            proc = subprocess.Popen(
                ["/usr/bin/socat", "-", f"TCP:{SSH_HOST}:{SSH_PORT}"],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
            )
        except FileNotFoundError:
            proc = None

    if proc is None:
        rfcomm_sock.sendall(b"ERROR: no SSH or socat available\r\n")
        rfcomm_sock.close()
        return

    rfcomm_sock.setblocking(False)
    proc.stdin = proc.stdin  # type: ignore

    def pipe(src, dst, close_dst_on_eof=True):
        try:
            while running:
                try:
                    data = src.recv(4096)
                except (AttributeError, BlockingIOError):
                    data = b""
                if isinstance(src, socket.socket):
                    try:
                        data = src.recv(4096)
                    except (BlockingIOError, socket.timeout):
                        data = b""
                if not data:
                    break
                dst.write(data)
                dst.flush()
        except (BrokenPipeError, ConnectionError, OSError):
            pass
        finally:
            if close_dst_on_eof:
                try:
                    dst.close()
                except Exception:
                    pass

    t1 = threading.Thread(target=lambda: pipe(rfcomm_sock, proc.stdin), daemon=True)
    t2 = threading.Thread(target=lambda: pipe(proc.stdout, rfcomm_sock, False), daemon=True)
    t1.start()
    t2.start()
    t1.join()
    t2.join()
    proc.terminate()
    proc.wait(5)


# ---- SPP Server (classic Bluetooth) ----
def spp_server():
    """RFCOMM SPP server using PyBluez."""
    try:
        import bluetooth
    except ImportError:
        print("PyBluez not available, SPP disabled", file=sys.stderr)
        return

    try:
        sock = bluetooth.BluetoothSocket(bluetooth.RFCOMM)
        sock.bind(("", SPP_CHANNEL))
        sock.listen(1)
        bluetooth.advertise_service(
            sock, "EdgeRun SSH",
            service_id="00001101-0000-1000-8000-00805F9B34FB",
            service_classes=["00001101-0000-1000-8000-00805F9B34FB"],
            profiles=[bluetooth.SERIAL_PORT_PROFILE],
        )
        print(f"SPP server listening on channel {SPP_CHANNEL}", file=sys.stderr)
    except Exception as e:
        print(f"SPP init failed: {e}", file=sys.stderr)
        return

    while running:
        try:
            client, addr = sock.accept()
            print(f"SPP connection from {addr}", file=sys.stderr)
            ssh_bridge(client)
        except Exception as e:
            if running:
                print(f"SPP accept error: {e}", file=sys.stderr)


# ---- BLE GATT Server (via BlueZ D-Bus) ----
class BLEApplication:
    """BlueZ D-Bus GATT application."""

    def __init__(self, bus):
        self.bus = bus
        self.path = "/com/edgerun/btbridge"
        self.service_path = f"{self.path}/service0"
        self.tx_char_path = f"{self.service_path}/char0"
        self.rx_char_path = f"{self.service_path}/char1"
        self._registered = False
        self._connections = []
        self._rx_handler = None

    def _xml_service(self):
        return f"""
<!DOCTYPE node PUBLIC "-//freedesktop//DTD D-BUS Object Introspection 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd">
<node>
  <interface name="org.freedesktop.DBus.Introspectable">
    <method name="Introspect">
      <arg name="xml" type="s" direction="out"/>
    </method>
  </interface>
  <interface name="org.bluez.GattService1">
    <property name="UUID" type="s" access="read"/>
    <property name="Primary" type="b" access="read"/>
    <property name="Includes" type="ao" access="read"/>
  </interface>
  <interface name="org.freedesktop.DBus.ObjectManager">
    <method name="GetManagedObjects">
      <arg name="objects" type="a{oa{sa{sv}}}" direction="out"/>
    </method>
  </interface>
  <node name="char0"/>
  <node name="char1"/>
</node>"""

    def _xml_tx_char(self):
        return f"""
<!DOCTYPE node PUBLIC "-//freedesktop//DTD D-BUS Object Introspection 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd">
<node>
  <interface name="org.freedesktop.DBus.Introspectable">
    <method name="Introspect">
      <arg name="xml" type="s" direction="out"/>
    </method>
  </interface>
  <interface name="org.bluez.GattCharacteristic1">
    <property name="UUID" type="s" access="read"/>
    <property name="Service" type="o" access="read"/>
    <property name="Flags" type="as" access="read"/>
    <method name="ReadValue">
      <arg name="options" type="a{{sv}}" direction="in"/>
      <arg name="value" type="ay" direction="out"/>
    </method>
    <method name="StartNotify"/>
    <method name="StopNotify"/>
    <signal name="PropertiesChanged">
      <arg name="interface" type="s"/>
      <arg name="changed" type="a{{sv}}"/>
      <arg name="invalidated" type="as"/>
    </signal>
  </interface>
</node>"""

    def _xml_rx_char(self):
        return f"""
<!DOCTYPE node PUBLIC "-//freedesktop//DTD D-BUS Object Introspection 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd">
<node>
  <interface name="org.freedesktop.DBus.Introspectable">
    <method name="Introspect">
      <arg name="xml" type="s" direction="out"/>
    </method>
  </interface>
  <interface name="org.bluez.GattCharacteristic1">
    <property name="UUID" type="s" access="read"/>
    <property name="Service" type="o" access="read"/>
    <property name="Flags" type="as" access="read"/>
    <method name="WriteValue">
      <arg name="value" type="ay" direction="in"/>
      <arg name="options" type="a{{sv}}" direction="in"/>
    </method>
    <method name="ReadValue">
      <arg name="options" type="a{{sv}}" direction="in"/>
      <arg name="value" type="ay" direction="out"/>
    </method>
  </interface>
</node>"""


class BLEBridge:
    """BLE GATT server using raw dbus integration."""

    def __init__(self):
        self.tx_data = bytearray()
        self.tx_lock = threading.Lock()
        self.rx_data = bytearray()
        self.rx_lock = threading.Lock()
        self.connected = False
        self._notify_enabled = False
        self._session_proc = None
        self._thread = None

    def on_rx_write(self, data: bytes):
        """Handle incoming data from BLE client (Web Bluetooth)."""
        with self.rx_lock:
            self.rx_data.extend(data)
        if self._thread is None or not self._thread.is_alive():
            self._thread = threading.Thread(target=self._session, daemon=True)
            self._thread.start()

    def get_tx_data(self) -> Optional[bytes]:
        """Get outgoing data for BLE client."""
        with self.tx_lock:
            if len(self.tx_data) == 0:
                return None
            data = bytes(self.tx_data)
            self.tx_data.clear()
            return data

    def _session(self):
        """Bridge BLE data to SSH."""
        proc = subprocess.Popen(
            ["/usr/bin/socat", "-", f"TCP:{SSH_HOST}:{SSH_PORT}"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        self._session_proc = proc
        self.connected = True

        poll = select.poll()
        poll.register(proc.stdout, select.POLLIN)

        while running and proc.poll() is None:
            with self.rx_lock:
                if len(self.rx_data) > 0:
                    data = bytes(self.rx_data)
                    self.rx_data.clear()
                    try:
                        proc.stdin.write(data)
                        proc.stdin.flush()
                    except BrokenPipeError:
                        break

            events = poll.poll(100)
            for fd, event in events:
                if event & select.POLLIN:
                    data = proc.stdout.read(4096)
                    if not data:
                        break
                    with self.tx_lock:
                        self.tx_data.extend(data)
                    if self._notify_enabled:
                        self._send_notification(data)

        self.connected = False
        self._session_proc = None

    def _send_notification(self, data: bytes):
        """Send BLE notification (D-Bus PropertiesChanged signal)."""
        try:
            import dbus
            bus = dbus.SystemBus()
            obj = bus.get_object(
                "org.bluez",
                "/com/edgerun/btbridge/service0/char0"
            )
            iface = dbus.Interface(obj, "org.freedesktop.DBus.Properties")
            iface.PropertiesChanged(
                "org.bluez.GattCharacteristic1",
                {"Value": dbus.Array([dbus.Byte(b) for b in data], signature="y")},
                [],
            )
        except Exception:
            pass


def ble_server():
    """Register BLE GATT service via BlueZ D-Bus."""
    try:
        import dbus
        import dbus.service
        import dbus.mainloop.glib
    except ImportError:
        print("dbus-python not available, BLE disabled", file=sys.stderr)
        return

    dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
    bus = dbus.SystemBus()

    bridge = BLEBridge()

    class GattApplication(dbus.service.Object):
        def __init__(self, bus, path):
            self.path = path
            dbus.service.Object.__init__(self, bus, path)

        @dbus.service.method("org.freedesktop.DBus.ObjectManager",
                             out_signature="a{oa{sa{sv}}}")
        def GetManagedObjects(self):
            return {
                self.path: {
                    "org.bluez.GattService1": {
                        "UUID": dbus.String(NUS_SERVICE_UUID),
                        "Primary": dbus.Boolean(True),
                        "Includes": dbus.Array([], "o"),
                    }
                },
                f"{self.path}/char0": {
                    "org.bluez.GattCharacteristic1": {
                        "UUID": dbus.String(NUS_TX_UUID),
                        "Service": dbus.ObjectPath(self.path),
                        "Flags": dbus.Array(["notify"], "s"),
                    }
                },
                f"{self.path}/char1": {
                    "org.bluez.GattCharacteristic1": {
                        "UUID": dbus.String(NUS_RX_UUID),
                        "Service": dbus.ObjectPath(self.path),
                        "Flags": dbus.Array(["write"], "s"),
                    }
                },
            }

        @dbus.service.method("org.freedesktop.DBus.Introspectable",
                             out_signature="s")
        def Introspect(self):
            return self._xml

    class TxCharacteristic(dbus.service.Object):
        def __init__(self, bus, path, service_path):
            self.service_path = service_path
            dbus.service.Object.__init__(self, bus, path)

        @dbus.service.method("org.bluez.GattCharacteristic1",
                             in_signature="a{sv}", out_signature="ay")
        def ReadValue(self, options):
            data = bridge.get_tx_data()
            if data is None:
                return dbus.Array([], "y")
            return dbus.Array([dbus.Byte(b) for b in data], "y")

        @dbus.service.method("org.bluez.GattCharacteristic1",
                             in_signature="a{sv}")
        def StartNotify(self):
            bridge._notify_enabled = True

        @dbus.service.method("org.bluez.GattCharacteristic1",
                             in_signature="a{sv}")
        def StopNotify(self):
            bridge._notify_enabled = False

    class RxCharacteristic(dbus.service.Object):
        def __init__(self, bus, path, service_path):
            self.service_path = service_path
            dbus.service.Object.__init__(self, bus, path)

        @dbus.service.method("org.bluez.GattCharacteristic1",
                             in_signature="aya{sv}")
        def WriteValue(self, value, options):
            data = bytes(value)
            bridge.on_rx_write(data)

        @dbus.service.method("org.bluez.GattCharacteristic1",
                             in_signature="a{sv}", out_signature="ay")
        def ReadValue(self, options):
            return dbus.Array([], "y")

    app_path = "/com/edgerun/btbridge"
    svc_path = f"{app_path}/service0"
    tx_path = f"{svc_path}/char0"
    rx_path = f"{svc_path}/char1"

    app = GattApplication(bus, app_path)
    svc = GattApplication(bus, svc_path)
    tx = TxCharacteristic(bus, tx_path, svc_path)
    rx = RxCharacteristic(bus, rx_path, svc_path)

    # Register with BlueZ
    mgr = bus.get_object("org.bluez", "/")
    adapter = mgr.FindAdapter("hci0", dbus_interface="org.bluez.Adapter1")
    adapter.SetDiscoveryFilter({})

    try:
        gatt_mgr = bus.get_object("org.bluez", "/org/bluez/hci0")
        gatt_mgr.RegisterApplication(
            app_path,
            {},
            dbus_interface="org.bluez.GattManager1",
        )
        print(f"BLE GATT service registered: {NUS_SERVICE_UUID}", file=sys.stderr)
    except Exception as e:
        print(f"BLE registration failed: {e}", file=sys.stderr)
        print("BLE disabled (may need bluetoothd --experimental)", file=sys.stderr)
        return

    import gi.repository.GLib
    loop = gi.repository.GLib.MainLoop()
    try:
        loop.run()
    except KeyboardInterrupt:
        loop.quit()


# ---- Main ----
def main():
    print("EdgeRun BT SSH Bridge v1", file=sys.stderr)
    print("Built from edgerun/standards protocol: bluetooth", file=sys.stderr)

    threads = []

    # Start SPP server
    spp_thread = threading.Thread(target=spp_server, daemon=True)
    spp_thread.start()
    threads.append(spp_thread)

    # Start BLE server (blocking, runs GLib loop)
    ble_thread = threading.Thread(target=ble_server, daemon=True)
    ble_thread.start()
    threads.append(ble_thread)

    signal.signal(signal.SIGINT, lambda s, f: sys.exit(0))
    signal.signal(signal.SIGTERM, lambda s, f: sys.exit(0))

    try:
        while running:
            time.sleep(1)
    except KeyboardInterrupt:
        pass
    finally:
        global running
        running = False


if __name__ == "__main__":
    main()
