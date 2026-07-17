#!/bin/sh
#
# Generate new tls key and cert for localhost development
#
# The certificate is self-signed for localhost, 0--1.nip.io (resolves to ::1), 127-0-0-1.nip.io
#  (resolves to 127.0.0.1), and fbi.com (resolves to 127.0.0.1).
openssl req -x509 -out localhost.crt -keyout localhost.key \
  -newkey rsa:4096 -noenc -sha256 \
  -subj '/CN=localhost' -extensions EXT -config <( \
   printf "[dn]\nCN=localhost\n[req]\ndistinguished_name = dn\n[EXT]\nsubjectAltName=DNS:localhost\nkeyUsage=digitalSignature\nextendedKeyUsage=serverAuth"
   printf "[EXT]\nsubjectAltName=DNS:0--1.nip.io\nkeyUsage=digitalSignature\nextendedKeyUsage=serverAuth"
   printf "[EXT]\nsubjectAltName=DNS:127-0-0-1.nip.io\nkeyUsage=digitalSignature\nextendedKeyUsage=serverAuth"
   printf "[EXT]\nsubjectAltName=DNS:fbi.com\nkeyUsage=digitalSignature\nextendedKeyUsage=serverAuth")
