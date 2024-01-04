## Overview
A small utility that adds some useful functionality for working with Subversion repositories.

```
$ svr -h
Subversion utilities

Usage: svr [COMMAND]

Commands:
  log       Display formatted log entries
  branch    Display current branch or list branches and tags
  show      Show the details of a commit
  filerevs  Display commit revisions of files across tags and branches
  stash     Stash away changes to a dirty working copy
  bisect    Use binary search to find the commit that introduced a bug
  prefix    Display and configure repository prefixes
  ignore    Write svn:ignore properties to stdout in .gitignore format
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

For help about a particular command type 'svr help COMMAND'
```
## License

```
  Copyright (c) 2023 Curt Sellmer
  
  Permission is hereby granted, free of charge, to any person obtaining
  a copy of this software and associated documentation files (the
  "Software"), to deal in the Software without restriction, including
  without limitation the rights to use, copy, modify, merge, publish,
  distribute, sublicense, and/or sell copies of the Software, and to
  permit persons to whom the Software is furnished to do so, subject to
  the following conditions:
  
  The above copyright notice and this permission notice shall be
  included in all copies or substantial portions of the Software.
  
  THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
  EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
  MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
  NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
  LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
  OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
  WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```
