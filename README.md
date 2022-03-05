# igtv-downloader

(Experimental) Instagram live stream downloader.
Can download livestreams from start.

## Usage

1. Play an ongoing Instagram livestream in desktop browser with a logged-in account.
    * **Note:** Instagram is known to disable accounts with bot-like behaviors, use a throwaway account to be safe.
2. Open network monitor by pressing F12 then navigate to the "Network" tab.
3. In the filter bar, type `.mpd`. Right click on one of the entries and select `Copy > Copy URL` (Firefox) or `Copy > Copy link address` (Chrome).
4. Run the downloader as show below with the .mpd url from step 3.

```console
$ ./igtv-downloader download 'https://url/to/manifest.mpd'
```
