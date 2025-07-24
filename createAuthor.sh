#!/bin/bash

# Script to create a new author in the Party Hub database
# Usage: ./createAuthor.sh <author_name>

set -e  # Exit on any error

# Check if author name is provided
if [ $# -eq 0 ]; then
    echo "Usage: $0 <author_name>"
    echo "Example: $0 Bob"
    exit 1
fi

AUTHOR_NAME="$1"
DB_FILE="party.db"

# Check if database file exists
if [ ! -f "$DB_FILE" ]; then
    echo "Error: Database file '$DB_FILE' not found."
    echo "Please run the application first to create the database, or run from the correct directory."
    exit 1
fi

# Generate a random UUID for the author ID
# Using a combination of date, random number, and process ID for uniqueness
AUTHOR_ID=$(cat /proc/sys/kernel/random/uuid 2>/dev/null || uuidgen 2>/dev/null || echo "$(date +%s)-$(($RANDOM * $$))-$(date +%N)")

# Generate a random secret (64 characters, alphanumeric)
AUTHOR_SECRET=$(openssl rand -hex 32 2>/dev/null || head -c 64 /dev/urandom | xxd -p | tr -d '\n' || echo "$(date +%s)$(($RANDOM))$(date +%N)" | sha256sum | cut -d' ' -f1)

# Insert the new author into the database
sqlite3 "$DB_FILE" "INSERT INTO authors (id, name, author_secret) VALUES ('$AUTHOR_ID', '$AUTHOR_NAME', '$AUTHOR_SECRET');"

if [ $? -eq 0 ]; then
    echo "✅ Author '$AUTHOR_NAME' created successfully!"
    echo "📋 Author ID: $AUTHOR_ID"
    echo "🔑 Author Secret: $AUTHOR_SECRET"
    echo ""
    echo "Save this secret! You'll need it to log in to the Party Hub management interface."
    echo "You can access the application at: http://localhost:8080"
else
    echo "❌ Failed to create author '$AUTHOR_NAME'"
    exit 1
fi
