#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
qemu_run_id="${QEMU_RUN_ID:-$$}"
qemu_log="${QEMU_LOG:-/tmp/edgerun-qemu-net-pump-${qemu_run_id}.log}"
qemu_net_dump="${QEMU_NET_DUMP:-/tmp/edgerun-qemu-net-pump-${qemu_run_id}.pcap}"
mcast_addr="${QEMU_SOCKET_MCAST_ADDR:-230.0.0.1}"
mcast_port="${QEMU_SOCKET_MCAST_PORT:-$((20000 + $$ % 20000))}"
start_wait_seconds="${QEMU_NET_PUMP_START_WAIT:-60}"

rm -f "$qemu_log" "$qemu_net_dump"

QEMU_TIMEOUT="${QEMU_TIMEOUT:-8}" \
QEMU_EXPECT="${QEMU_EXPECT:-ICMP echo reply sent}" \
QEMU_LOG="$qemu_log" \
QEMU_NET_DUMP="$qemu_net_dump" \
QEMU_NETDEV="socket,id=n0,mcast=${mcast_addr}:${mcast_port}" \
    "$repo_root/scripts/qemu-unikernel.sh" &
qemu_pid=$!

cleanup() {
    wait "$qemu_pid" 2>/dev/null || true
}
trap cleanup EXIT

for _ in $(seq 1 $((start_wait_seconds * 10))); do
    if grep -q "Net pump started" "$qemu_log" 2>/dev/null; then
        break
    fi
    if ! kill -0 "$qemu_pid" 2>/dev/null; then
        wait "$qemu_pid" 2>/dev/null || true
        echo "QEMU exited before net pump start" >&2
        echo "Serial log: $qemu_log" >&2
        exit 1
    fi
    sleep 0.1
done

if ! grep -q "Net pump started" "$qemu_log" 2>/dev/null; then
    echo "QEMU did not start net pump before injection" >&2
    exit 1
fi

python3 - "$mcast_addr" "$mcast_port" <<'PY'
import socket
import struct
import sys
import time

group = (sys.argv[1], int(sys.argv[2]))
guest_mac = bytes.fromhex("525400123456")
host_mac = bytes.fromhex("020000000001")
host_ip = bytes([192, 168, 1, 1])
guest_ip = bytes([192, 168, 1, 12])


def checksum(data):
    if len(data) & 1:
        data += b"\x00"
    total = sum((data[i] << 8) + data[i + 1] for i in range(0, len(data), 2))
    while total >> 16:
        total = (total & 0xffff) + (total >> 16)
    return (~total) & 0xffff


def arp_request():
    arp = (
        struct.pack("!HHBBH", 1, 0x0800, 6, 4, 1)
        + host_mac
        + host_ip
        + b"\x00" * 6
        + guest_ip
    )
    return b"\xff" * 6 + host_mac + struct.pack("!H", 0x0806) + arp


def icmp_echo():
    payload = b"edgerun"
    icmp = struct.pack("!BBHHH", 8, 0, 0, 7, 1) + payload
    icmp = icmp[:2] + struct.pack("!H", checksum(icmp)) + icmp[4:]
    ip = struct.pack(
        "!BBHHHBBH4s4s",
        0x45,
        0,
        20 + len(icmp),
        1,
        0,
        64,
        1,
        0,
        host_ip,
        guest_ip,
    )
    ip = ip[:10] + struct.pack("!H", checksum(ip)) + ip[12:]
    return guest_mac + host_mac + struct.pack("!H", 0x0800) + ip + icmp


sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
sock.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_TTL, 1)

for frame in (arp_request(), icmp_echo()):
    for _ in range(5):
        sock.sendto(frame, group)
        time.sleep(0.05)
PY

wait "$qemu_pid"
trap - EXIT

if grep -q "ARP reply sent" "$qemu_log"; then
    echo "QEMU net pump smoke reached marker: ARP reply sent"
else
    echo "QEMU net pump smoke did not reach marker: ARP reply sent" >&2
    echo "Serial log: $qemu_log" >&2
    exit 1
fi
