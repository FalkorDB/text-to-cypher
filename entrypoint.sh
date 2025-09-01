#!/bin/bash

echo "Starting supervisord with process management..."

# Ensure import directory exists and has correct permissions
mkdir -p /var/lib/falkordb/import
chown -R appuser:appuser /var/lib/falkordb/import
chmod -R 755 /var/lib/falkordb/import

# Ensure FalkorDB data directory permissions
chown -R appuser:appuser /var/lib/falkordb
chmod -R 755 /var/lib/falkordb

# Start supervisord - it will manage all processes with integrated log formatting
exec /usr/bin/supervisord -c /etc/supervisor/conf.d/supervisord.conf
