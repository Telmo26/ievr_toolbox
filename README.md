# IE VR Toolbox

This program aims at being a collection of utilities for making modding Inazuma Eleven Victory Road easier. The code is heavily based on both [Viola](https://github.com/SuperTavor/Viola) and [CriFsV2Lib](https://github.com/Sewer56/CriFsV2Lib), but re-written in Rust for performance. On top of that, heavy parallelism is used to make the best possible use of one's computer's resources.

![Screenshot of the program](.github/assets/screenshot.png)

The selective dumping is possible thanks to my work on [IEVR Cfg Bin Editor](https://github.com/Telmo26/ievr_cfg_bin_editor) that is currently a work in progress.

# Features

- **Performance**: This is the main reason for this tool's existence, and its only use as of now. The tool dumps game files significantly faster than Viola. However, the dumping process is heavily I/O-bound, so the faster your storage the faster the program will go. Conversely, a slow HDD will probably not see a huge difference in performance. Based on my own testing, this tool is now around 3 times faster than Viola for a full dump.

- **Partial dumping**: This tool supports selecting the files you want to dump. To do so, first create a text file. Each line of the text file must contain a valid [regular expression](https://en.wikipedia.org/wiki/Regular_expression) (REGEX). Every file from the game whose filename (not the directory!) matches one of the regular expressions will be extracted. You then pass the text file to the program using the `-r` or `--rules-file` argument. For example, a text file containing 
    ```
    ^chara_.*\.cfg\.bin$
    ^skill_.*\.cfg\.bin$
    ```
  will extract all cfg.bin files that start with either "chara_" or "skill_". Of course, you need to know the files' names ahead of time to be able to use it.
  If you don’t know what REGEX is, you can safely ignore this feature and do a full dump. If you still want to use it, AI tools are very good at generating REGEXes.

# Usage

It is available for both Windows and Linux. Rust compiles static binaries, so the Linux version should be compatible with most x86_64 distributions. 

The tool is only available as a CLI tool for now, as I prefer focusing on adding features before making a GUI. This means there is no user interface to speak of, as the program is only usable in the terminal. For Windows users, you can open a terminal in a folder by right-clicking on empty space and selecting "Open in the terminal".

During extraction, high disk usage is normal and the program may take several minutes depending on your storage speed.

## Basic

The only required option is the input folder, selected using the `-i` or `--input-folder` option. It takes the game's root folder as an input. The most basic command is therefore 
```
.\ievr_toolbox-cli-win64.exe dump -i "path/to/the/game/folder"
```
This will extract the full game files into the "extracted" folder next to the binary. You can specify an output folder through `-o` or `--output-folder` using the same syntax.

## Advanced

A help menu is available by opening a terminal in the folder you downloaded the file and typing `ievr_toolbox-linux64 -h` (Linux) or `.\ievr_toolbox-win64.exe -h` (Windows). On top of the previously mentioned options, there are 3 more:

- The `-t` or `--threads` option specifies how many threads you want the program to use. Usually, unless your storage is very slow, more threads is faster, so the default is set to all available threads.
- The `-m` or `--memory` option specifies the maximum amount of RAM you allow the program to use. In the same way, having more memory is faster, so the default is to use all the available memory.
- The `-r` or `--rules-file` option specifies the aforementionned file that contains the REGEX rules. The file must contain one valid REGEX rule per line, and the program only filters based on the filename, not the directory path.

# AI disclosure
AI was used extensively for this project, mainly to help me understand the purpose of some of the code from the original libraries, since my knowledge of C# is pretty limited.