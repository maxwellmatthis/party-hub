#!/bin/bash

# Script to create a party from a YAML file
# Usage: ./create_party.sh party_config.yaml

if [ $# -eq 0 ]; then
    echo "Usage: $0 party_config.yaml"
    echo ""
    echo "YAML format example:"
    echo "party:"
    echo "  id: party-123"
    echo "  invitation_blocks:"
    echo "    - template: text_input"
    echo "      content: \"What food allergies do you have?\""
    echo "    - template: multiple_choice"
    echo "      content:"
    echo "        label: \"Will you attend the ceremony?\""
    echo "        options:"
    echo "          - \"Yes\""
    echo "          - \"No\""
    echo "          - \"Maybe\""
    echo "    - template: number_input"
    echo "      content: \"How many guests will you bring?\""
    echo "guests:"
    echo "  - guest-id-1"
    echo "  - guest-id-2"
    exit 1
fi

YAML_FILE="$1"

# Check if YAML file exists
if [ ! -f "$YAML_FILE" ]; then
    echo "Error: YAML file '$YAML_FILE' not found."
    exit 1
fi

# Check if database exists
if [ ! -f "party.db" ]; then
    echo "Error: party.db not found. Please run the server first to create the database."
    exit 1
fi

# Check if yq is installed for YAML parsing
if ! command -v yq &> /dev/null; then
    echo "Error: yq is required to parse YAML files."
    echo "Install it with: brew install yq"
    exit 1
fi

# Parse YAML and extract data
PARTY_ID=$(yq eval '.party.id' "$YAML_FILE")
GUEST_IDS=($(yq eval '.guests[]' "$YAML_FILE"))

echo "Creating party: $PARTY_ID"
echo "Guest IDs: ${GUEST_IDS[@]}"

# Convert invitation blocks to JSON
INVITATION_BLOCKS_JSON=$(yq eval '.party.invitation_blocks' "$YAML_FILE" -o=json)

if [ "$INVITATION_BLOCKS_JSON" = "null" ] || [ -z "$INVITATION_BLOCKS_JSON" ]; then
    echo "Error: No invitation_blocks found in YAML file"
    exit 1
fi

# Compact the JSON (remove unnecessary whitespace)
if command -v jq &> /dev/null; then
    INVITATION_BLOCKS_JSON=$(echo "$INVITATION_BLOCKS_JSON" | jq -c '.')
fi

echo "Invitation blocks JSON: $INVITATION_BLOCKS_JSON"

# Escape single quotes in JSON for SQL
ESCAPED_JSON=$(echo "$INVITATION_BLOCKS_JSON" | sed "s/'/''/g")

# Check if party already exists
PARTY_EXISTS=$(sqlite3 party.db "SELECT COUNT(*) FROM parties WHERE id = '$PARTY_ID';")
if [ "$PARTY_EXISTS" -gt 0 ]; then
    echo "⚠ Warning: Party with ID '$PARTY_ID' already exists."
    read -p "Do you want to update it? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        sqlite3 party.db "UPDATE parties SET invitation_blocks = '$ESCAPED_JSON' WHERE id = '$PARTY_ID';"
        echo "✓ Party updated successfully!"
    else
        echo "Operation cancelled."
        exit 0
    fi
else
    # Insert party
    sqlite3 party.db "INSERT INTO parties (id, invitation_blocks) VALUES ('$PARTY_ID', '$ESCAPED_JSON');"
    if [ $? -ne 0 ]; then
        echo "✗ Failed to create party"
        exit 1
    fi
    echo "✓ Party created successfully!"
fi

# Create invitations for each guest
INVITATION_COUNT=0
for GUEST_ID in "${GUEST_IDS[@]}"; do
    INVITATION_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
    
    # Check if guest exists
    GUEST_EXISTS=$(sqlite3 party.db "SELECT COUNT(*) FROM guests WHERE id = '$GUEST_ID';")
    if [ "$GUEST_EXISTS" -eq 0 ]; then
        echo "⚠ Warning: Guest ID '$GUEST_ID' not found in database. Skipping invitation."
        continue
    fi
    
    # Check if invitation already exists for this guest and party
    EXISTING_INVITATION=$(sqlite3 party.db "SELECT COUNT(*) FROM invitations WHERE guest_id = '$GUEST_ID' AND party_id = '$PARTY_ID';")
    if [ "$EXISTING_INVITATION" -gt 0 ]; then
        echo "⚠ Warning: Invitation already exists for guest $GUEST_ID. Skipping."
        continue
    fi
    
    # Create empty answers object based on number of blocks
    BLOCK_COUNT=$(yq eval '.party.invitation_blocks | length' "$YAML_FILE")
    EMPTY_ANSWERS="{}"
    
    sqlite3 party.db "INSERT INTO invitations (id, guest_id, party_id, status, invitation_block_answers) VALUES ('$INVITATION_ID', '$GUEST_ID', '$PARTY_ID', 'pending', '$EMPTY_ANSWERS');"
    
    if [ $? -eq 0 ]; then
        echo "✓ Invitation created for guest $GUEST_ID (ID: $INVITATION_ID)"
        ((INVITATION_COUNT++))
    else
        echo "✗ Failed to create invitation for guest $GUEST_ID"
    fi
done

echo ""
echo "🎉 Party setup complete!"
echo "  Party ID: $PARTY_ID"
echo "  Invitations created: $INVITATION_COUNT"
echo ""
for GUEST_ID in "${GUEST_IDS[@]}"; do
    INVITATION_ID=$(sqlite3 party.db "SELECT id FROM invitations WHERE guest_id = '$GUEST_ID' AND party_id = '$PARTY_ID' ORDER BY rowid DESC LIMIT 1;")
    if [ -n "$INVITATION_ID" ]; then
        GUEST_NAME=$(sqlite3 party.db "SELECT name FROM guests WHERE id = '$GUEST_ID';")
        echo "$GUEST_NAME: http://127.0.0.1:8080/$INVITATION_ID"
    fi
done
