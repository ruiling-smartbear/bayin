# Reference recordings

Real instruments and animals, for measuring against. `./fetch.sh` downloads them; they are not in
the repository and are not redistributed. Bayin uses numbers measured from them (pitches,
lengths, harmonic levels), not the audio. All are on Wikimedia Commons; thank you to the people
who recorded and shared them.

| Name | For | Recording | By | License |
|---|---|---|---|---|
| assembly | bugle | [Assembly bugle call.ogg](https://commons.wikimedia.org/wiki/File:Assembly_bugle_call.ogg) | Sgt. Codie Lynn Williams, U.S. Marine Corps | Public domain |
| taps | bugle | [Taps on bugle.ogg](https://commons.wikimedia.org/wiki/File:Taps_on_bugle.ogg) | United States Army Band | Public domain |
| k5la | air horn | [AirChime K5LA.ogg](https://commons.wikimedia.org/wiki/File:AirChime_K5LA.ogg) | HarveyHenkelmann | CC0 |
| nathan-m5 | air horn | [Nathan M5.ogg](https://commons.wikimedia.org/wiki/File:Nathan_M5.ogg) | HarveyHenkelmann | CC0 |
| up844 | steam whistle | [2010 UP844 Whistle.ogg](https://commons.wikimedia.org/wiki/File:2010_UP844_Whistle.ogg) | Rvassar | CC BY-SA 3.0 |
| c57 | steam whistle | [C57180.kiteki.1.ogg](https://commons.wikimedia.org/wiki/File:C57180.kiteki.1.ogg) | SL Story's | CC BY-SA 4.0 |
| dog-bark | dog | [Barking of a dog.ogg](https://commons.wikimedia.org/wiki/File:Barking_of_a_dog.ogg) | Amada44 | CC BY-SA 3.0 |
| dog-bark2 | dog | [Barking of a dog 2.ogg](https://commons.wikimedia.org/wiki/File:Barking_of_a_dog_2.ogg) | Amada44 | CC BY-SA 3.0 |
| dog-growl, dog-growl2 | dog | Audio S2 and S3 of "Dogs' Expectation about Signalers' Body Size by Virtue of Their Growls", PLoS ONE 5(12): e15175 (2010), on Commons | T. Faragó, P. Pongrácz, Á. Miklósi, L. Huber, Z. Virányi, F. Range | CC BY 2.5 |
| dog-howl | dog | [Jem howls.ogg](https://commons.wikimedia.org/wiki/File:Jem_howls.ogg) | amethysta (PDSounds) | Public domain |

Measure a recording of steady notes with `python3 tools/measure.py notes references/assembly/assembly.wav`,
and calls that are not steady notes with `python3 tools/contour.py references/dog-bark/dog-bark.wav`.
