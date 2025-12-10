![Slowfetch Logo](https://raw.githubusercontent.com/tuibird/Slowfetch/refs/heads/master/slowfetch.png)

A fetch program for my girlfriend. She doesnt rice, So the goal was a terminal that looks riced for her out of the box.

I'm mainly doing this for the sake of learning and honestly making a fetch program with "pretty" defaults sounds good to me.

As far as the hardware it supports, I run CashyOS which is Arch based. So its primarily designed around my setup. Expect weirdness as I iron out bugs with OS/Hardware support.

I will be adding features as i go, but this is no means stable. Excpect some breaking changes, even in main.

## Documentation

The Argument `--os` can be used to display the OS art instead of the Slowfetch logo. For debug purposes you can force a distro using a name following the argument. example: `--os arch`
The amount of supported of support OSs is currently small as I haven't settled on an art style yet.

As of v0.2.5 you can pass images with the argument `-i` followed by a path `~/Pictures.examplepath.png`.
This is very early stages so it is currently built around 1x1 aspect pictures. kitty image protocol does all the scaling so i reccomend sizing your images appropiatly for the terminal size you are expecting, 2000x2000 pixel pics will work but the scaling will make em look not great.

Since V0.2.3 there is now a config file! Should be placed at `~/.config/slowfetch/config.toml`. Currently you can change the launch options for which art to display (sorry no custom art yet!). You can also change the colors used for the interface and modify the ascii art palette. The default config can be found in `src/config.toml`. As with everything else here, expect bugs.

## Contributing

I currently won't accept PR's as this defeats the whole point of the project (sorry!).

## Installation

To install Slowfetch, pull the source and use the following command from the root of the project.

`cargo install --path .`

## Example of the program and its dynamic layout

![Slowfetch Screenshot](https://raw.githubusercontent.com/tuibird/Slowfetch/refs/heads/master/slowfetch0-2-5.png))