<div align="center">

# 🔎 TextLint 🔎
**An OS-wide, AI-powered writing assistant for Windows**
</div>

## What is TextLint?

TextLint is a minimal, always-on Windows 11 desktop utility that lives in your system tray. Powered by Tauri v2 and the blazing-fast Gemini Flash Lite API.

Wherever you're typing — any app, any text field — you can select text, hit a global shortcut (`Ctrl+Alt+G`), and instantly get:

1. **The corrected text**, auto-pasted back into your text field (optional).
2. **An explanation panel** popping up right where you are typing, showing what was wrong, why it was wrong, and what the correct form is — helping you learn and improve over time.

## ✨ Features

- **⚡ System-Wide Integration**: Works in *any* Windows application where text can be selected.
- **🚀 Ultra-Fast**: Powered by Rust, Tauri, and Google's Gemini Flash Lite API.
- **🎯 Smart Positioning**: The correction panel magically appears right above your cursor.
- **🧠 Learn Mode**: Don't just fix it—understand it. Get detailed explanations of your mistakes.
- **✍️ Enhance Writing**: Go beyond grammar — improve word choice, clarity, and sentence flow with a single toggle.
- **🎨 Fluent Design**: Gorgeous native WinUI 3 aesthetics with light/dark theme support.
- **🎛️ App Filtering**: Easily disable TextLint in specific apps (like VS Code or Terminal).
- **🔒 Privacy First**: Lightweight, brings-your-own-API-key model.

## 🛠️ Stack

- **Frontend**: Vanilla TypeScript/HTML/CSS (Zero framework overhead for maximum performance)
- **Backend / Core**: Rust
- **Framework**: [Tauri v2](https://v2.tauri.app/)
- **AI Engine**: Google Gemini Flash Lite API (`gemini-flash-lite-latest`)

## 🚀 Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) (v16+)
- [Rust](https://rustup.rs/) (latest stable)
- Visual Studio 2022 C++ Build Tools (Required by Tauri on Windows)

### Installation & Development

1. **Clone the repository**
   ```bash
   git clone https://github.com/yourusername/textlint.git
   cd textlint
   ```

2. **Install frontend dependencies**
   ```bash
   npm install
   ```

3. **Run in development mode**
   ```bash
   npm run tauri dev
   ```

4. **Setup API Key**
   - The app will start in your system tray.
   - Right-click the system tray icon and select **Settings**.
   - Input your [Google Gemini API Key](https://aistudio.google.com/app/apikey).
   - Test by selecting some text and pressing `Ctrl+Alt+G`.

### Building for Production

To build a standalone `.exe` and `.msi` installer:

```bash
npm run tauri build
```

## ⌨️ How to Use

1. **Select text** anywhere (Slack, Chrome, Word, etc).
2. Wait a split second to ensure text is fully highlighted.
3. Press `Ctrl + Alt + G`.
4. Wait ~1 second for the Gemini API.
5. Watch as your text is auto-corrected (if Auto-Apply is on) and an explanation panel appears next to your cursor!

## 🤝 Contributing

Contributions, issues, and feature requests are welcome!

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
