# UrTask: Trello to Telegram Daily Reporter 🚀📊

## 🌟 Overview

UrTask is an automated tool designed to streamline the process of daily reporting for individuals and teams using Trello for task management. This application bridges the gap between Trello boards and Telegram, providing users with daily summaries of their Trello activities directly through a Telegram bot.

## 🔑 Key Features

- 🤖 **Automated Daily Reports**: Fetches updates from user-specified Trello boards and sends a concise summary to the user via Telegram.
- 📋 **Task Categorization**: Organizes tasks into "To Do", "Doing", and "Done" categories for clear progress tracking.
- ⏰ **Customizable Reporting Time**: Users can set their preferred time to receive daily reports.
- 🔄 **Easy Sharing**: Facilitates easy forwarding of reports to company channels or groups for team-wide updates.
- 🔒 **Secure Authentication**: Utilizes Trello API keys and tokens for secure access to user boards.
- 📊 **On-Demand Reports**: Get immediate task reports with the /reportnow command.

## 🔧 How It Works

1. 🛠️ **Setup**: Users configure their Trello board details and Telegram chat ID through the UrTask bot.
2. 🔍 **Daily Scan**: UrTask scans the specified Trello board at a scheduled time each day.
3. 📝 **Report Generation**: It compiles a report of tasks moved to "Done" since the last report, current "Doing" tasks, and upcoming "To Do" items.
4. 📬 **Delivery**: The generated report is sent to the user via the UrTask Telegram bot.
5. 📢 **Sharing (Optional)**: Users can easily forward the report to their team or company reporting channels.

## 🛠️ Technical Stack

- 🦀 **Backend**: Rust
- 🔌 **APIs**: Trello API, Telegram Bot API
- 🗄️ **Database**: MongoDB (for storing user configurations and log data)
- ⚙️ **Scheduling**: Tokio for asynchronous task scheduling

## 🚀 Getting Started

### Prerequisites

- 🦀 Rust (latest stable version)
- 🗄️ MongoDB
- 📋 A Trello account with API key and token
- 🤖 A Telegram bot token

### Installation and Setup

1. Clone the repository:
   ```
   git clone https://github.com/yourusername/urtask.git
   cd urtask
   ```

2. Copy the `.env.sample` file to `.env` and fill in your actual values:
   ```
   cp .env.sample .env
   ```

3. Edit the `.env` file with your specific configuration:
   ```
   MONGODB_CONNECTION_STRING=your_mongodb_connection_string
   URTASK_BOT_TOKEN=your_telegram_bot_token
   ```

4. Install dependencies and run:
   ```
   cargo build
   cargo run
   ```

### Using the Bot

1. Start a chat with your Telegram bot.
2. Use the `/setconfig` command followed by your Trello board ID, API key, and token:
   ```
   /setconfig your_board_id-your_api_key-your_api_token
   ```
3. Available commands:
   - `/start` - Get started with the bot
   - `/help` - Show all available commands
   - `/setconfig` - Configure your Trello integration
   - `/testconfig` - Test your configuration
   - `/reportnow` - Get an immediate report of your tasks
4. The bot will automatically send daily reports at scheduled times.

## 🔧 Troubleshooting

- 🔔 If you're not receiving messages, check your Telegram bot token and ensure the bot has been started in your Telegram client.
- 🔌 For database connection issues, verify your MongoDB connection string in the `.env` file.
- 🔑 Make sure your Trello API key and token have the necessary permissions to access your board.

For more detailed troubleshooting, check the application logs or file an issue on the GitHub repository.

## 🤝 Contributing

We welcome contributions to UrTask! Please read our contributing guidelines to get started.

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

---

🚀 Automate your daily reports and boost your productivity with UrTask!