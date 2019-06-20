#!/bin/sh

sudo mkdir -p /opt/dpdk/build
sudo mkdir -p /opt/dpdk/build/include
sudo mkdir -p /opt/dpdk/build/lib
sudo mkdir -p /opt/dpdk/build/include/exec-env
sudo mkdir -p /opt/dpdk/build/include/generic

sudo rm -rf ~/trash/*

src=~/tools/dpdk-stable-17.08.1/build/lib
dst=/opt/dpdk/build/lib

#clean destination
if [ -d $dst ]; then
   echo "Cleaning target directory $dst"
   sudo mv $dst/* ~/trash/
fi

sudo cp -r $src/* $dst

src=~/tools/dpdk-stable-17.08.1/build/include
dst=/opt/dpdk/build/include

#clean destination
if [ -d $dst ]; then
   echo "Cleaning target directory $dst"
   sudo mv $dst/* ~/trash/
fi

#copy files
for i in `ls $src`; do
   path=""
   if [  -L $src/$i ]; then
      path=`readlink -f $src/$i`
      #echo $src/$i
      #echo $path
      sudo ln -s $path $dst/$i
   else
      sudo cp -r -vP $src/$i $dst
   fi
done


src=~/tools/dpdk-stable-17.08.1/build/include/exec-env
dst=/opt/dpdk/build/include/exec-env

#clean destination
if [ -d $dst ]; then
   echo "Cleaning target directory $dst"
   sudo mv $dst/* ~/trash/
fi

#copy files
for i in `ls $src`; do
   path=""
   if [  -L $src/$i ]; then
      path=`readlink -f $src/$i`
      #echo $src/$i
      #echo $path
      sudo ln -s $path $dst/$i
   else
      sudo cp -r -vP $src/$i $dst
   fi
done


src=~/tools/dpdk-stable-17.08.1/build/include/generic
dst=/opt/dpdk/build/include/generic
#clean destination
if [ -d $dst ]; then
   echo "Cleaning target directory $dst"
   sudo mv $dst/* ~/trash/
fi

#copy files
for i in `ls $src`; do
   path=""
   if [  -L $src/$i ]; then
      path=`readlink -f $src/$i`
      #echo $src/$i
      #echo $path
      sudo ln -s $path $dst/$i
   else
      sudo cp -r -vP $src/$i $dst
   fi
done

# Listing target directiry files
#sudo ls -al $dst

