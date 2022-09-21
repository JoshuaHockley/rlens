# rlens default configuration

This directory contains a default configuration for rlens.\
The lua rc is responsible for setting all keybinds, so without one rlens is useless.
You could use it as a template, or start the rc from scratch.

### Default keybinds
These are the keybinds set by `rc.lua`.

#### General
| Key | Action                              |
| :-- | :---------------------------------- |
| `q` | Exit rlens                          |
| `f` | Toggle fullscreen                   |
| `d` | Toggle the status bar in image mode |
| `R` | Reload the current image            |

#### Mode changing
| Key      | Action                                       |
| :------- | :------------------------------------------- |
| `Tab`    | Toggle between image mode and gallery mode   |
| `Return` | Open the current gallery image in image mode |

#### Image mode navigation
| Key          | Action                   |
| :----------- | :----------------------- |
| `Left`, `p`  | Go to the previous image |
| `Right`, `n` | Go to the next image     |
| `g`          | Go to the first image    |
| `G`          | Go to the last image     |

#### Gallery mode navigation
| Key          | Action                |
| :----------- | :-------------------- |
| `Left`, `h`  | Move the cursor left  |
| `Right`, `l` | Move the cursor right |
| `Up`, `k`    | Move the cursor up    |
| `Down`, `j`  | Move the cursor down  |
| `g`          | Go to the first image |
| `G`          | Go to the last image  |

#### Image transforms
| Key      | Action                    |
| :------- | :------------------------ |
| `h`      | Pan left                  |
| `l`      | Pan right                 |
| `k`      | Pan up                    |
| `j`      | Pan down                  |
| `i`, `=` | Zoom in                   |
| `o`, `-` | Zoom out                  |
| `,`      | Rotate counter-clockwise  |
| `.`      | Rotate clockwise          |
| `b`      | Flip horizontally         |
| `v`      | Flip vertically           |
| `r`      | Reset the image transform |
| `s`      | Change scaling mode       |

