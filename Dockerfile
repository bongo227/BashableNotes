FROM ubuntu:latest
RUN apt-get update
RUN apt-get install -y python python-pip python-tk
RUN pip install matplotlib numpy
# Change matplotlib backend to non-interactive
RUN mkdir -p $HOME/.config/matplotlib/
RUN echo "backend : Agg" >> $HOME/.config/matplotlib/matplotlibrc
