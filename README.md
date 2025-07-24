# Party Hub

A modern, minimalistic party invitation platform built with Rust (Actix Web), SQLite, and vanilla JavaScript. Create beautiful party invitations with custom questions, gather responses, and display public statistics in real-time.

![Party Management Interface](examples/Party-Hub-Management.png)
*Management interface for creating parties and managing guests*

![Guest Invitation View](examples/Party-Hub-Invitation.png)
*Guest-facing invitation with real-time statistics*

## Features

- 🎉 **Custom Party Creation**: Create parties with flexible block-based invitation editor
- 📝 **Multiple Question Types**:
  - **Text Input**: Free-form text responses
  - **Number Input**: Numeric responses
  - **Single Choice**: Radio button selections with live vote counts
  - **Multiple Choice**: Checkbox selections with live vote counts
- 👥 **Guest Management**: Easy guest creation, invitation system, and organizer privileges
- 📊 **Real-time Statistics**: Public questions show live vote counts and guest names as responses come in
- 🔒 **Privacy Controls**: Questions can be marked as public (visible to all) or private (organizer-only)
- 📱 **Responsive Design**: Modern, clean UI that works on all devices
- ⚡ **Live Updates**: Vote counts update instantly when you make selections
- 🌐 **Localization**:
  - Automatic language detection based on browser preferences
  - Full German and English support
  - Informal German ("du") for friendly communication
- 👤 **Name Personalization**: Use `{{name}}` in content to personalize invitations
- 🔐 **Secure Authentication**: Token-based authentication for party organizers

## Quick Start

### Prerequisites

- Rust (latest stable version)
- SQLite3

### Installation & Setup

1. **Clone and run**:

   ```bash
   git clone https://github.com/maxwellmatthis/party-hub.git
   cd party-hub
   cargo run
   ```

   The server will start on `http://localhost:8080`

2. **Create your first author account**:

   ```bash
   ./createAuthor.sh "Your Name"
   ```

   Save the generated secret to log into the management interface.

## License

This project is licensed under the **GNU Affero General Public License v3.0** (AGPLv3). See the LICENSE file for details.
