# Slowfetch

Made this fetch program for myself and my girlfriend. I'm mainly doing this for the sake of learning and honestly making a fetch program with "pretty" out of the box defaults while I'm at it.

Currently it displays a short list of system information and the slowfetch logo art. It will adjust its layout (to an extent, wide/narrow) depending on the size of your terminal on execution. I don't actually know if its slow, i mean it feels kinda fast? But I think the name is funny so im sticking with it.

As far as the hardware it supports, I run CashyOS which is Arch based. So its primarily designed around my setup. I've tested it on a good few terminals and ive tried to include compatibility for some common package managers like dpkg and rpm, but i never actually got around to testing them so there is non zero chance it panics or something stupid.

Will be adding features slowly when as i feel like it.

## Documentation

Currently there is no configuration of any sort, that is on the list of things to re-implement. In version one of this project I had a config.toml system for full configuration. However during the rewrite I decided to cut this out to focus on making it as fast as possible.

## Contributing

I currently won't accept PR's as this defeats the whole point of the project (sorry!).

## Installation

To install Slowfetch, pull the source and use the following command from the root of the project.

`cargo install --path .`

## Example of the program and It's dynamic layout

![Slowfetch Screenshot](https://raw.githubusercontent.com/tuibird/Slowfetch/7a2145e931b85e2cea1d083e107514867aa169e9/example.png)
