# Party Hub

A modern, minimalistic party invitation platform built with Rust (Actix Web), SQLite, and vanilla JavaScript. Create beautiful party invitations with custom questions, gather responses, and display public statistics in real-time.

## Features

- 🎉 **Custom Party Creation**: Create parties with flexible question types
- 📝 **Multiple Question Types**: Text input, number input, single choice (radio), and multiple choice (checkbox)
- 👥 **Guest Management**: Easy guest creation and invitation system
- 📊 **Real-time Statistics**: Public questions show live vote counts as guests respond
- 🔒 **Privacy Controls**: Questions can be marked as public or private
- 📱 **Responsive Design**: Modern, clean UI that works on all devices
- ⚡ **Live Updates**: Vote counts update instantly when you make selections
- 🌐 **Localization**: Automatic language detection with German and English support
- 👤 **Name Personalization**: Use `{{name}}` in content to personalize invitations

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [jq](https://jqlang.github.io/jq/) - JSON processor for command line
- [yq](https://github.com/mikefarah/yq) - YAML processor for command line

Install dependencies:

```bash
# macOS
brew install jq yq

# Ubuntu/Debian
sudo apt install jq
sudo wget -qO /usr/local/bin/yq https://github.com/mikefarah/yq/releases/latest/download/yq_linux_amd64
sudo chmod +x /usr/local/bin/yq

# Windows (using chocolatey)
choco install jq yq
```

### Installation & Setup

1. **Clone the repository**:

   ```bash
   git clone https://github.com/maxwellmatthis/party-hub.git
   cd party-hub
   ```

2. **Build and run the server**:

   ```bash
   cargo run
   ```

   The server will start on `http://localhost:8080`

3. **Create guests and get their IDs**:

   ```bash
   ./create_guest.sh "Alice Johnson"
   # Output: Created guest 'Alice Johnson' with ID: 354a58d2-814e-45bb-9be7-5bdd895f87d8
   
   ./create_guest.sh "Bob Smith"
   # Output: Created guest 'Bob Smith' with ID: 943f16e9-3de9-4810-8282-e7853649b083
   
   ./create_guest.sh "Carol Davis"
   # Output: Created guest 'Carol Davis' with ID: 46addae4-c880-4895-803f-87500829c3f6
   ```

   **Note**: Copy these UUIDs for use in your party YAML file.

4. **Create your first party** (see examples in `examples/` directory):

   ```bash
   ./create_party.sh examples/test_public_party.yaml
   ```

5. **Visit the invitation** in your browser:

   ```text
   http://localhost:8080/<invitation-id>
   ```

## Usage Guide

### Creating a Party

Create a YAML file describing your party (see `examples/` for templates):

```yaml
party:
  id: "my-awesome-party"
  blocks:
    - template: "h1"
      content: "My Awesome Party!"
    
    - template: "p"
      content: "Join us for an amazing celebration!"
    
    - template: "multiple_choice"
      content: |
        {
          "label": "Will you attend?",
          "options": ["Yes", "No", "Maybe"],
          "public": true
        }
    
    - template: "text_input"
      content: |
        {
          "label": "Any dietary restrictions?",
          "public": false
        }

guests:
  - "354a58d2-814e-45bb-9be7-5bdd895f87d8"  # Alice's guest ID
  - "943f16e9-3de9-4810-8282-e7853649b083"  # Bob's guest ID
  - "46addae4-c880-4895-803f-87500829c3f6"  # Carol's guest ID
```

**Important**: The `guests` section must contain guest UUIDs, not names. Create guests first with `./create_guest.sh "Name"` and use the returned UUIDs.

Then create the party:

```bash
./create_party.sh my-party.yaml
```

### Question Types

#### Text Input

```yaml
- template: "text_input"
  content: |
    {
      "label": "Your message here",
      "public": true
    }
```

#### Number Input

```yaml
- template: "number_input"
  content: |
    {
      "label": "How many guests will you bring?",
      "public": false
    }
```

#### Single Choice (Radio buttons)

```yaml
- template: "single_choice"
  content: |
    {
      "label": "Choose your meal preference",
      "options": ["Vegetarian", "Meat", "Vegan"],
      "public": true
    }
```

#### Multiple Choice (Checkboxes)

```yaml
- template: "multiple_choice"
  content: |
    {
      "label": "Which activities interest you?",
      "options": ["Dancing", "Games", "Karaoke", "Food"],
      "public": true
    }
```

#### Text Content

```yaml
- template: "h1"
  content: "Main Heading"

- template: "h2"
  content: "Subheading"

- template: "p"
  content: "Regular paragraph text"

- template: "code"
  content: "Code or monospace text"
```

### Name Personalization

You can personalize your invitation content by using `{{name}}` anywhere in your text content. This will be automatically replaced with the guest's name when they view their invitation.

```yaml
- template: "h1"
  content: "Welcome to the party, {{name}}!"

- template: "p"
  content: "Hi {{name}}, we're excited to celebrate with you!"

- template: "text_input"
  content: |
    {
      "label": "{{name}}, what's your favorite music genre?",
      "public": false
    }

- template: "single_choice"
  content: |
    {
      "label": "{{name}}, will you attend our party?",
      "options": ["Yes, I'll be there!", "Sorry, can't make it"],
      "public": true
    }
```

**Note**: Name personalization works in all text content, including headers, paragraphs, and question labels, but not in option text or other structured data.

### Localization

The application automatically detects the user's preferred language from their browser's `Accept-Language` header:

- **German users** (browsers set to German): See the interface in German
- **All other users**: See the interface in English (default)

The system supports:

- German (`de`, `de-DE`, `de-AT`, etc.) → German interface
- All other languages → English interface (fallback)

Language detection includes:

- Status messages ("Speichern" vs "Save")
- Error messages ("Ein Fehler ist aufgetreten" vs "An error occurred")
- UI elements ("Andere Rückmeldungen" vs "Other responses")

### Privacy Settings

- **Public questions** (`"public": true`): Responses are visible to all guests with vote counts
- **Private questions** (`"public": false`): Responses are only visible to the party organizer

### Managing Guests

Create individual guests:

```bash
./create_guest.sh "Full Name"
```

Or include guests directly in your party YAML file under the `guests` section (using guest UUIDs, not names).

### Viewing Responses

Guests can:

- View their invitation at `http://localhost:8080/<invitation-id>`
- See real-time statistics for public questions
- Save their responses (automatically updates vote counts)

## Example Workflows

### 1. Birthday Party

```bash
# Create guests first and note their IDs
./create_guest.sh "Alice Johnson"
./create_guest.sh "Bob Smith" 
./create_guest.sh "Carol Davis"

# Update examples/test_public_party.yaml with the guest IDs from above
# Then create the party
./create_party.sh examples/test_public_party.yaml

# The script will output invitation URLs for each guest
# Send these URLs to your guests via email, text, etc.
```

### 2. Personalized Birthday Party

Create a personalized invitation using name templates:

```yaml
party:
  id: "sarah-birthday-2025"
  blocks:
    - template: "h1"
      content: "You're Invited, {{name}}!"
    
    - template: "p"
      content: "Hi {{name}}, Sarah is turning 25 and we want you to celebrate with us!"
    
    - template: "single_choice"
      content: |
        {
          "label": "{{name}}, will you join us for the celebration?",
          "options": ["Yes, I'll be there!", "Sorry, can't make it"],
          "public": true
        }
    
    - template: "text_input"
      content: |
        {
          "label": "{{name}}, any song requests for the party playlist?",
          "public": false
        }

guests:
  - "guest-uuid-alice"
  - "guest-uuid-bob"
  - "guest-uuid-carol"
```

When Alice views her invitation, she'll see "You're Invited, Alice!" and "Alice, will you join us for the celebration?" - making each invitation feel personally crafted.

### 3. Testing Different Question Types

```bash
# Create a test party with various question types
./create_party.sh examples/test_choice_types.yaml

# Add some test data to see how statistics work
sqlite3 party.db < examples/test_choice_data.sql
```

### 4. Corporate Event

```bash
# Create guests first
./create_guest.sh "John Doe"
./create_guest.sh "Jane Smith"

# Use the guest IDs output from the commands above in your party YAML
# For example, if the guest IDs are:
# John Doe: a1b2c3d4-e5f6-7890-abcd-ef1234567890
# Jane Smith: b2c3d4e5-f6g7-8901-bcde-f23456789012

# Then create your party YAML file with these IDs:
# guests:
#   - "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
#   - "b2c3d4e5-f6g7-8901-bcde-f23456789012"

./create_party.sh my-corporate-event.yaml
```

## File Structure

```text
party-hub/
├── src/
│   ├── main.rs          # Main server code
│   └── db.rs            # Database models
├── static/
│   ├── index_en.html    # English HTML templates and UI text
│   ├── index_de.html    # German HTML templates and UI text
│   ├── main.js          # Frontend JavaScript (MVC)
│   └── style.css        # Responsive CSS
├── examples/            # Example parties and test data
├── create_party.sh      # Party creation script
├── create_guest.sh      # Guest creation script
└── party.db            # SQLite database (created automatically)
```

## Development

### Database Schema

The application uses SQLite with three main tables:

- **parties**: Store party definitions and question blocks
- **guests**: Store guest information
- **invitations**: Link guests to parties and store their responses

### Frontend Architecture

The frontend uses a clean MVC (Model-View-Controller) pattern:

- **Model**: Manages invitation data and answers
- **View**: Handles rendering and DOM manipulation
- **Controller**: Coordinates between model and view, handles API calls

### API Endpoints

- `GET /details/{invitation_id}` - Fetch invitation data, guest name, and responses
- `POST /details/{invitation_id}` - Save guest responses
- `GET /static/{file}` - Serve static files
- `GET /{invitation_id}` - Serve invitation page (automatically localized based on Accept-Language header)

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is open source and available under the AGPLv3 License.

## Troubleshooting

### Common Issues

**Server won't start**: Check if port 8080 is already in use:

```bash
lsof -i :8080
```

**Scripts fail**: Ensure jq and yq are installed and executable:

```bash
which jq yq
jq --version
yq --version
```

**Database issues**: Delete `party.db` to start fresh:

```bash
rm party.db
cargo run  # Will recreate the database
```

**Permission issues**: Make scripts executable:

```bash
chmod +x create_party.sh create_guest.sh
```

For more help, check the `examples/` directory for working party definitions and test data.
