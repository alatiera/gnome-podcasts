## Contributing

Contributing

When contributing to the development of Hammond, please first discuss the change you wish to make via issue, email, or any other method with the maintainers before making a change.

Please note we have a code of conduct, please follow it in all your interactions with the project.

## Style

We use rustfmt for code formatting and we enforce it on the gitlab-CI server.

Quick setup
   ```
   cargo install rustfmt-nightly
   cargo fmt --all
   ```

It is recommended to add a pre-commit hook to run cargo test and cargo fmt
   ```
   #!/bin/sh
   cargo test --all && cargo fmt --all -- --write-mode=diff
   ```

## Pull Request Process

1. Ensure your code compiles. Run `make` before creating the pull request.
2. If you're adding new API, it must be properly documented.
3. The commit message is formatted as follows:
   ```
   component: <summary>

   A paragraph explaining the problem and its context.

   Another one explaining how you solved that.

   <link to the bug ticket>
   ```
4. You may merge the pull request in once you have the sign-off of the maintainers, or if you
   do not have permission to do that, you may request the second reviewer to merge it for you.

## Code of Conduct
We follow the Gnome [Code of Conduct.](https://wiki.gnome.org/Foundation/CodeOfConduct)
