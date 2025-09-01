#!/bin/bash

echo "Starting supervisord with process management..."

# Ensure import directory exists and has correct permissions
mkdir -p /var/lib/FalkorDB/import
chown -R appuser:appuser /var/lib/FalkorDB/import
chmod -R 755 /var/lib/FalkorDB/import

# Ensure FalkorDB data directory permissions (both cases)
chown -R appuser:appuser /var/lib/falkordb
chmod -R 755 /var/lib/falkordb
chown -R appuser:appuser /var/lib/FalkorDB
chmod -R 755 /var/lib/FalkorDB

# Create a temporary directory for any additional imports FalkorDB might need
mkdir -p /tmp/falkordb-import
chown -R appuser:appuser /tmp/falkordb-import
chmod -R 755 /tmp/falkordb-import

# Start supervisord - it will manage all processes with integrated log formatting
exec /usr/bin/supervisord -c /etc/supervisor/conf.d/supervisord.conf
