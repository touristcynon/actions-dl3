## update and install some things we should probably have
apt-get update
apt-get install -y \
  curl \
  git \
  jq \
  zsh \
  vim \
  build-essential \
  openssl

## Install rustup and common components
curl https://sh.rustup.rs -sSf | sh -s -- -y 
cargo install cargo-expand

## install golang
curl -sSfLO https://dl.google.com/go/go1.20.1.linux-amd64.tar.gz
 rm -rf /usr/local/go && tar -C /usr/local -xzf go1.20.1.linux-amd64.tar.gz
 export PATH=$PATH:/usr/local/go/bin
 go version

## setup and install oh-my-zsh
sh -c "$(curl -fsSL https://raw.githubusercontent.com/robbyrussell/oh-my-zsh/master/tools/install.sh)"
cp -R /root/.oh-my-zsh /home/$USERNAME
cp /root/.zshrc /home/$USERNAME
sed -i -e "s/\/root\/.oh-my-zsh/\/home\/$USERNAME\/.oh-my-zsh/g" /home/$USERNAME/.zshrc
chown -R $USER_UID:$USER_GID /home/$USERNAME/.oh-my-zsh /home/$USERNAME/.zshrc
