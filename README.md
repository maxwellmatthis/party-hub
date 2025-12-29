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
  - **Attendance Block**: Dedicated RSVP functionality with customizable options
- 👥 **Guest Management**: Easy guest creation, invitation system, and organizer privileges
- 🔗 **Public Party Links**: Share a single link that allows anyone to self-register and create their own invitation
  - Copy public party link with one click
  - Self-registered guests are marked with a badge
  - Optional guest limit enforcement
- 📊 **Real-time Statistics**: Public questions show live vote counts and guest names as responses come in
- 🔒 **Privacy Controls**: Questions can be marked as public (visible to all) or private (organizer-only)
- 📱 **Responsive Design**: Modern, clean UI that works on all devices
- ⚡ **Live Updates**: Vote counts update instantly when you make selections
- 🌐 **Localization**:
  - Automatic language detection based on browser preferences
  - Full German and English support
  - Informal German ("du") for friendly communication
- 👤 **Guest Personalization**: Use template variables in your content:
  - `{{salutation}}` - Guest's salutation (e.g., Mr., Ms., Dr.)
  - `{{first}}` - Guest's first name
  - `{{last}}` - Guest's last name
  - `{{name}}` - Full name (first + last) for backward compatibility
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

### Environment Variables

Party Hub supports the following environment variables for configuration:

- **`PORT`**: Set the server port (default: 8080)

  ```bash
  export PORT=3000
  cargo run
  ```

- **`ENV`**: Set the environment mode (default: "prod")
  - `ENV=dev`: Enables development mode with insecure cookies for localhost testing
  - `ENV=prod`: Production mode with secure cookies (default)

  ```bash
  export ENV=dev
  cargo run
  ```

- **`MAIL_SENDTYPE`**: Choose email sending method (optional)
  - `MAIL_SENDTYPE=client`: Use SMTP client (send through mail provider)
  - `MAIL_SENDTYPE=direct`: Use direct SMTP (send directly to recipient's server)
  - If not set: Automatically detects and uses available method (client preferred)

  ```bash
  # Force client mode
  export MAIL_SENDTYPE=client
  
  # Force direct mode
  export MAIL_SENDTYPE=direct
  cargo run
  ```

  ```bash
  export ENV=dev
  cargo run
  ```

### VAPID Keys Setup

1. **Generate keys**:

```bash
# Install web-push CLI globally
npm install -g web-push

# Generate VAPID keys
web-push generate-vapid-keys
```

2. **Save keys** in the project root:

```bash
echo "YOUR_PRIVATE_KEY_HERE" > private_vapid_key.pem
echo "YOUR_PUBLIC_KEY_HERE" > public_vapid_key.pem
```

### Email Notifications Setup (Optional)

Party Hub supports two methods for sending email notifications to guests:

#### Option 1: SMTP Client (Using your email provider)

This method sends emails through your email provider's SMTP server (Gmail, Outlook, etc.). This is the easiest option but requires storing your email credentials on the server.

1. **Set environment variables**:

```bash
export SMTP_SERVER="smtp.gmail.com"              # Your SMTP server
export SMTP_USERNAME="your-email@gmail.com"      # Your email address
export SMTP_PASSWORD="your-app-password"         # Your email password or app password
export SMTP_FROM="Party Hub <noreply@yourdomain.com>"  # From address
export BASE_URL="https://your-domain.com"        # Base URL for invitation links
```

2. **Common SMTP providers**:

- **Gmail**: 
  - Server: `smtp.gmail.com`
  - Port: 587 (automatic)
  - Requires an [App Password](https://support.google.com/accounts/answer/185833)
  
- **Outlook/Office 365**:
  - Server: `smtp.office365.com`
  - Port: 587 (automatic)
  
- **Custom/Self-hosted**:
  - Use your mail server's SMTP details

#### Option 2: Direct SMTP (Send directly from your server)

This method sends emails directly to recipients' mail servers without going through your email provider. This means **you don't need to store your email credentials on the server**, but it requires proper DNS configuration to avoid spam filters.

1. **Set only the required environment variables**:

```bash
export SMTP_FROM="Party Hub <noreply@yourdomain.com>"  # Must match your domain
export BASE_URL="https://your-domain.com"        # Base URL for invitation links
```

**Note**: To force direct SMTP mode even when client credentials are available, set `MAIL_SENDTYPE=direct`. Otherwise, if SMTP client credentials are not configured, Party Hub will automatically use direct SMTP sending.

2. **DNS Configuration for Direct SMTP**:

To ensure your emails aren't marked as spam, you must configure these DNS records for your domain:

**SPF Record** (Sender Policy Framework):
```
Type: TXT
Name: @
Value: v=spf1 ip4:YOUR_SERVER_IP -all
```

Replace `YOUR_SERVER_IP` with your server's public IP address. This tells email servers that your server is authorized to send email for your domain.

**DKIM Record** (DomainKeys Identified Mail):
DKIM signing is more complex and requires generating a private/public key pair. Consider using a service like [EasyDMARC](https://easydmarc.com/tools/dkim-record-generator) to generate your DKIM keys and records.

**DMARC Record** (Domain-based Message Authentication):
```
Type: TXT
Name: _dmarc
Value: v=DMARC1; p=quarantine; rua=mailto:postmaster@yourdomain.com
```

This tells receiving servers to quarantine emails that fail SPF/DKIM checks and send reports to your postmaster address.

**Example complete DNS setup**:
```
# SPF - Authorize your server's IP
@ IN TXT "v=spf1 ip4:203.0.113.10 -all"

# DMARC - Set policy and reporting
_dmarc IN TXT "v=DMARC1; p=quarantine; rua=mailto:postmaster@yourdomain.com"

# DKIM - Public key for verification (generated separately)
default._domainkey IN TXT "v=DKIM1; k=rsa; p=YOUR_PUBLIC_KEY_HERE"
```

**Important Notes**:
- DNS changes can take up to 48 hours to propagate
- Test your configuration with [mail-tester.com](https://www.mail-tester.com/)
- Without proper SPF/DKIM/DMARC, your emails will likely end up in spam
- Major email providers (Gmail, Outlook) require DKIM for good deliverability
- Consider using Option 1 (SMTP Client) if DNS configuration is too complex

3. **Testing email**:

```bash
# Test with SMTP Client
export SMTP_SERVER="smtp.gmail.com"
export SMTP_USERNAME="test@gmail.com"
export SMTP_PASSWORD="your-app-password"
export SMTP_FROM="Party Hub <noreply@example.com>"
export MAIL_SENDTYPE="client"  # Optional: force client mode
cargo run

# Test with Direct SMTP (no credentials needed)
unset SMTP_SERVER SMTP_USERNAME SMTP_PASSWORD
export SMTP_FROM="Party Hub <noreply@yourdomain.com>"
export BASE_URL="https://your-domain.com"
export MAIL_SENDTYPE="direct"  # Optional: force direct mode
cargo run

# Add a guest and invite them to see email in action
```

**Note**: 
- Use `MAIL_SENDTYPE` to explicitly choose between `client` or `direct` email methods
- If `MAIL_SENDTYPE` is not set, Party Hub automatically detects and uses the configured method (prefers client over direct)
- If no email method is configured, Party Hub will log a warning at startup and continue without email notifications
- Push notifications will always work regardless of email configuration

## License

This project is licensed under the **GNU Affero General Public License v3.0** (AGPLv3). See the LICENSE file for details.
