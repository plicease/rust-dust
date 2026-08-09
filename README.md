# dust

directory dusting

# SYNOPSIS

    % dust
    % dust -d

# DESCRIPTION

Calculate the size of all child directories and list
in order of size, in human notation.

Pass `-d` to list directories only (when no explicit paths are given).
Pass explicit file/directory names to size just those; use `--` to stop
option parsing if a name begins with `-`.

# BUILD / INSTALL

    % cargo install --path .

This installs the `dust` binary (built from this Cargo project).

# AUTHOR

Graham Ollis <plicease@cpan.org>

# COPYRIGHT AND LICENSE

This software is copyright (c) 2013 by Graham Ollis.

This is free software; you can redistribute it and/or modify it under
the same terms as the Perl 5 programming language system itself.
