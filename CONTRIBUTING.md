## Contributing

First, thank you for contributing! Pull requests are always welcome!

### Git Commits

Please ensure your commits are small and focused; they should tell a story of your change. This helps reviewers to follow your changes, especially for more complex changes.

### Github Pull Requests

Once your changes are ready, submit your branch as a pull request.

The pull request title must follow the format outlined in the [conventional commits spec](https://www.conventionalcommits.org):

```
<type>: <description>
```

[Conventional commits](https://www.conventionalcommits.org) is a standardized format for commit messages. We use it to update the Changelog with each commit message and upgrade the package version.

As we squashes commits before merging branches, only the pull request title must conform to this format.

The following are all good examples of pull request titles:

```text
feat: add for bar baz feature
fix: fix foo bar baz bug
chore: improve build process
docs: fix typos
```

If your commit introduce a breaking change, append a `!` after the `type`:

```text
chore!: drop support for foo bar baz
```

Thanks! :heart: :heart: :heart:

The Klaxit Team
