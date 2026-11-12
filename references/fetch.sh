#!/bin/sh
# Download the reference recordings (see SOURCES.md) into references/<name>/ and convert each to
# 48 kHz mono WAV. They are for measurement only and are not part of the repository.
# Needs curl and ffmpeg.
set -e
cd "$(dirname "$0")"
fetch() {
  mkdir -p "$1"
  curl -sL -A "bayin-reference-fetch/1.0" -o "$1/$1.ogg" "https://upload.wikimedia.org/wikipedia/commons/$2"
  ffmpeg -v error -y -i "$1/$1.ogg" -ac 1 -ar 48000 "$1/$1.wav"
  echo "$1"
}
fetch assembly  2/21/Assembly_bugle_call.ogg
fetch taps      2/25/Taps_on_bugle.ogg
fetch k5la      8/89/AirChime_K5LA.ogg
fetch nathan-m5 3/35/Nathan_M5.ogg
fetch up844     4/4d/2010_UP844_Whistle.ogg
fetch c57       2/2f/C57180.kiteki.1.ogg
fetch dog-bark   a/a2/Barking_of_a_dog.ogg
fetch dog-bark2  5/58/Barking_of_a_dog_2.ogg
fetch dog-growl  5/5d/Dogs%27-Expectation-about-Signalers%27-Body-Size-by-Virtue-of-Their-Growls-pone.0015175.s006.ogg
fetch dog-growl2 4/44/Dogs%27-Expectation-about-Signalers%27-Body-Size-by-Virtue-of-Their-Growls-pone.0015175.s007.ogg
fetch dog-howl   c/c2/Jem_howls.ogg
