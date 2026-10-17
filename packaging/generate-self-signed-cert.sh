#!/usr/bin/env bash
#
# Generates the self-signed certificate the packaged nginx config points at.
#
# This is a placeholder so the site can be served over TLS before a real
# certificate exists. Browsers will warn on it. Replace it with a certificate
# from a CA (certbot, for example) by overwriting the same two paths, or by
# pointing ssl_certificate/ssl_certificate_key somewhere else and reloading.
#
#   sudo ./generate-self-signed-cert.sh [hostname-or-ip ...]
#
# Every argument is added as a subject alternative name. With no arguments the
# machine's hostname, localhost and the primary IPv4 address are used.
set -euo pipefail

cert_dir=/etc/nginx/ssl
cert=$cert_dir/wedding-website.crt
key=$cert_dir/wedding-website.key
days=825

if [ "$(id -u)" -ne 0 ]; then
    echo "Run this as root: it writes to $cert_dir." >&2
    exit 1
fi

names=("$@")
if [ ${#names[@]} -eq 0 ]; then
    names=("$(hostname -f 2>/dev/null || hostname)" localhost)
    primary_ip=$(ip -4 -o addr show scope global | awk 'NR==1 {sub(/\/.*/, "", $4); print $4}')
    [ -n "$primary_ip" ] && names+=("$primary_ip")
fi

# openssl wants IP addresses under IP: and everything else under DNS:.
sans=()
for name in "${names[@]}"; do
    if [[ $name =~ ^[0-9]+(\.[0-9]+){3}$ ]]; then
        sans+=("IP:$name")
    else
        sans+=("DNS:$name")
    fi
done
sans+=("IP:127.0.0.1")

# Duplicates are legal but noisy; a machine whose hostname is already localhost
# would otherwise list it twice.
san_list=$(printf '%s\n' "${sans[@]}" | awk '!seen[$0]++' | paste -sd,)

if [ -e "$cert" ] || [ -e "$key" ]; then
    read -r -p "Overwrite the existing certificate at $cert? [y/N] " reply
    case "$reply" in
        [yY]*) ;;
        *) echo "Left the existing certificate alone."; exit 0 ;;
    esac
fi

install -dm755 "$cert_dir"

openssl req -x509 -newkey rsa:2048 -nodes \
    -keyout "$key" \
    -out "$cert" \
    -days "$days" \
    -subj "/CN=${names[0]}" \
    -addext "subjectAltName=$san_list" \
    -addext "basicConstraints=critical,CA:FALSE" \
    -addext "keyUsage=critical,digitalSignature,keyEncipherment" \
    -addext "extendedKeyUsage=serverAuth"

chmod 600 "$key"
chmod 644 "$cert"

echo
echo "Wrote $cert and $key"
echo "Valid for $days days, covering: $san_list"
echo "Reload nginx to pick it up: systemctl reload nginx"
