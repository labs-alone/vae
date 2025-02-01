# Contributing to VAE

<div align="center">
  <img src="https://media.discordapp.net/attachments/1199307897344114738/1335200374637854782/Add_a_heading_2.png" alt="VAE Banner" width="100%">
</div>

First off, thank you for considering contributing to VAE! It's people like you that make VAE such a great tool.

## Code of Conduct

By participating in this project, you are expected to uphold our Code of Conduct. Please report unacceptable behavior to team@vae-engine.dev.

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check the issue list as you might find out that you don't need to create one. When you are creating a bug report, please include as many details as possible:

* Use a clear and descriptive title
* Describe the exact steps which reproduce the problem
* Provide specific examples to demonstrate the steps
* Describe the behavior you observed after following the steps
* Explain which behavior you expected to see instead and why
* Include screenshots if possible

### Suggesting Enhancements

If you have a suggestion for the project, we'd love to hear it. Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion, please include:

* A clear and descriptive title
* A detailed description of the proposed enhancement
* Examples of how the enhancement would be used
* Any potential drawbacks or challenges

### Pull Requests

1. Fork the repo and create your branch from `main`
2. If you've added code that should be tested, add tests
3. If you've changed APIs, update the documentation
4. Ensure the test suite passes
5. Make sure your code lints
6. Issue that pull request!

### Development Process

1. Clone the repository
```bash
git clone https://github.com/vae-engine/vae.git
cd vae
```

2. Create a branch
```bash
git checkout -b feature/your-feature-name
```

3. Make your changes
   - Write meaningful commit messages
   - Follow the coding style
   - Add tests if needed
   - Update documentation

4. Push to your fork and submit a pull request

## Styleguides

### Git Commit Messages

* Use the present tense ("Add feature" not "Added feature")
* Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
* Limit the first line to 72 characters or less
* Reference issues and pull requests liberally after the first line
* Follow conventional commits format:
    * feat: for new features
    * fix: for bug fixes
    * docs: for documentation changes
    * style: for formatting changes
    * refactor: for code refactoring
    * test: for adding tests
    * chore: for maintenance tasks

### Rust Style Guide

* Follow the official [Rust Style Guide](https://rust-lang.github.io/api-guidelines/)
* Use `rustfmt` to format your code
* Run `clippy` before submitting PRs
* Document public APIs
* Write meaningful variable names
* Keep functions focused and small

### Documentation Style Guide

* Use Markdown for documentation
* Include code examples when relevant
* Keep line length to 80 characters
* Use clear and concise language
* Update README.md if needed

## Project Structure

```
vae/
├── src/
│   ├── core/        # Core engine components
│   ├── vision/      # Vision processing modules
│   ├── models/      # ML model management
│   ├── runtime/     # Runtime optimization
│   └── api/         # External interfaces
├── tests/           # Test files
├── docs/            # Documentation
└── examples/        # Example implementations
```

## Getting Help

If you need help, you can:
- Join our [Discord community](https://discord.gg/vae-engine)
- Check the [documentation](docs/)
- Ask in GitHub issues
- Email the team at team@vae-engine.dev

## Recognition

Contributors are recognized in several ways:
- Listed in our [Contributors](CONTRIBUTORS.md) file
- Mentioned in release notes
- Given credit in documentation
- Awarded special Discord roles

Thank you for contributing to VAE!