# download-iglive

(Experimental) Instagram live stream downloader.
Can download live streams from start.

## Usage

1. Play an ongoing Instagram live stream in desktop browser with a logged-in account.
    * **Note:** Instagram is known to disable accounts with bot-like behaviors, use a throwaway account to be safe.
2. Open network monitor by pressing F12 then navigate to the "Network" tab.
3. In the filter bar, type `.mpd`. Right click on one of the entries and select `Copy > Copy URL` (Firefox) or `Copy > Copy link address` (Chrome).
4. Run the downloader as show below with the .mpd url from step 3.

```console
$ ./download-iglive download 'https://url/to/manifest.mpd'
```
## Examples

#### Specify download directory

```console
$ ./download-iglive download -o path/to/download/directory 'https://url/to/manifest.mpd'
```

#### Only download live segments (don't scrape past segments)

```console
$ ./download-iglive download -l 'https://url/to/manifest.mpd'
```

#### Merge already-downloaded segments into one video file

```console
$ ./download-iglive merge path/to/download/directory
```

#### View help

```console
$ ./download-iglive -h
```

```console
$ ./download-iglive download -h
```

```console
$ ./download-iglive merge -h
```
