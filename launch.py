#!/usr/bin/env python3

import subprocess
import sys
from argparse import ArgumentParser, ArgumentDefaultsHelpFormatter


def sh(command):
    if isinstance(command, str):
        command = command.split()
    print(*command)
    out = subprocess.run(
        command, stdout=subprocess.PIPE, stderr=sys.stderr, encoding='utf-8'
    )
    return out.stdout


def main(count: int, app: str):
    for i in range(count):
        sh(f'mkfifo /tmp/netchat-fifo-{i}')

    for i in range(count):
        j = (i + 1) % count
        sh(
            [
                'x-terminal-emulator',
                '-e',
                app.format(IN=f"/tmp/netchat-fifo-{i}", OUT=f"/tmp/netchat-fifo-{j}"),
            ]
        )


if __name__ == '__main__':
    p = ArgumentParser(
        description="Generate a network named pipes (FIFO files) and launch the applications.",
        formatter_class=ArgumentDefaultsHelpFormatter,
    )
    p.add_argument(
        'count', type=int, help="The number of nodes in the network (must be >= 2)"
    )
    p.add_argument(
        '--app',
        default="cargo run -- --input {IN} --output {OUT}",
        help="The application to be launched",
    )
    args = p.parse_args()
    assert args.count >= 2, "The number of nodes must be >= 2"
    main(args.count, args.app)
