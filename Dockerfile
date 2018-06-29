FROM ubuntu

LABEL maintainer="Tim Visee <timvisee@gmail.com>"

RUN apt update -y
RUN apt install git vim iputils-ping iputils-tracepath wget curl cowsay fortune lolcat toilet -y

CMD ["/bin/bash"]
