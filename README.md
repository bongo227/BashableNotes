# BashableNotes

Bashable notes allows you to run arbitary commands (inside a customizable docker container) on blocks of code and display their output. Think jupyter notebooks but simpler and more flexable.

Note, this project is still in _very_ early development.

## Code block options
Code block options are written in json after specifying a language
    
    ```python {/* Insert options here */}
    print("Hello world!")
    ```
    
The options avalible are:

- `name`: if a file name is provided, the file is saved inside the docker container
- `cmd`: the command to run, `stdout` and `stderr` will be displayed bellow the codeblock
- (more to come)

## Code blocks in action

### Custom docker container

By default all commands are run inside the `ubuntu:latest` docker container, if you need additional dependencies just create a new docker file (with `{"name":"Dockerfile"}`).

```dockerfile {"name":"Dockerfile"}
FROM ubuntu:latest
RUN apt-get update
RUN apt-get install -y python python-pip python-tk
RUN pip install matplotlib numpy
# Change matplotlib backend to non-interactive
RUN mkdir -p $HOME/.config/matplotlib/
RUN echo "backend : Agg" >> $HOME/.config/matplotlib/matplotlibrc
```

### Running python

Running python is as simple as naming the code block, and setting `cmd` (i.e. `{"cmd":"python file.py"`)

```python {"name":"helloworld.py", "cmd":"python helloworld.py"}
print("Hello world!")
```

### Image outputs

Want to show some `matplotlib` graphs? Simple save the file then use the markdown image syntax to insert the file.

```python {"name":"graph.py", "cmd":"python graph.py"}
import matplotlib.pyplot as plt
import numpy as np

plt.figure(figsize=[6,6])
x = np.arange(0,100,0.00001)
y = x*np.sin(2*np.pi*x)
plt.plot(y)
plt.axis('off')
plt.gca().set_position([0, 0, 1, 1])
plt.savefig("graph.svg")
```

![graph](notebook/graph.svg)

## Upcoming features

- File system to view markdown files in folder/subfolders.
- Stream code output instead of waiting execution to terminate
- Parse enviroment variables in via code block options
- HTML form controls set enviroment variables (enabling _interactive_ notebooks)
- Togglable dark theme
- Document overview
- Spinners on queued/executing code
- Ability to stop/pause/resume code execution
- Export options (i.e. markdown (with output), pdf, latex etc)