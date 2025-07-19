#!/bin/bash

# Script to create a guest in the party database
# Usage: ./create_guest.sh "Guest Name"

if [ $# -eq 0 ]; then
    echo "Usage: $0 \"Guest Name\""
    echo "Example: $0 \"John Doe\""
    exit 1
fi

GUEST_NAME="$1"
GUEST_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')

# Check if database exists
if [ ! -f "party.db" ]; then
    echo "Error: party.db not found. Please run the server first to create the database."
    exit 1
fi

# Insert the guest
sqlite3 party.db "INSERT INTO guests (id, name) VALUES ('$GUEST_ID', '$GUEST_NAME');"

if [ $? -eq 0 ]; then
    echo "✓ Guest created successfully!"
    echo "  ID: $GUEST_ID"
    echo "  Name: $GUEST_NAME"
else
    echo "✗ Failed to create guest"
    exit 1
fi
