#!/bin/bash

echo "Starting supervisord with process management..."

# Start supervisord - it will manage all processes with integrated log formatting
exec /usr/bin/supervisord -c /etc/supervisor/conf.d/supervisord.conf
