"""The corpus's 13 pages (docs/corpus.md §2, CO3, CO10).

URLs here are as the repository records them, without Prism's `u=` and
Figma's `t=` (CO13). The session by hand reads the full URLs from
scratch/corpus-urls.txt, which git ignores.
"""

from dataclasses import dataclass


@dataclass(frozen=True)
class Page:
    ident: str
    label: str
    tier: str
    url: str
    automated: bool  # corpus CO3: the site's terms and robots file permit automated loads
    logged_out: bool  # whether the page is worth measuring logged out (CO10)
    signed_in: bool  # whether a login changes or unlocks it (§2)
    interaction: str  # what is done for 30 s (CO10)


PAGES = [
    Page("google", "Google (search results)", "content", "https://www.google.com/search?q=Premier+League",
         False, True, True, "Scroll the results to the end, and back"),
    Page("youtube", "YouTube (video player and comments)", "media", "https://www.youtube.com/watch?v=2Zhv7SjAKeg",
         False, True, True, "Play the video; then scroll down until comments load"),
    Page("instagram", "Instagram (media feed)", "content", "https://www.instagram.com/leomessi",
         False, True, True, "Scroll the profile's grid five screens; open one post and close it"),
    Page("reddit", "Reddit (community feed)", "content", "https://www.reddit.com/r/RedditGames/",
         False, True, True, "Scroll the feed five screens"),
    Page("wikipedia", "Wikipedia (article layout and math)", "document", "https://en.wikipedia.org/wiki/Pi",
         True, True, False, "Scroll the article to the end at a steady pace, and back"),
    Page("maps", "Google Maps (interactive map tiles)", "media and maps",
         "https://www.google.com/maps/search/Times+Square,+New+York+City,+USA",
         True, True, False, "Pan the map with three drags; zoom in two steps and out two"),
    Page("twitch", "Twitch (live stream and chat)", "media", "https://www.twitch.tv/eslcs",
         False, True, True, "Watch the stream with the chat open"),
    Page("discord", "Discord (web client and channels)", "application",
         "https://discord.com/channels/662267976984297473/999550150705954856",
         False, False, True, "Scroll the channel's history up five screens, then back to the latest messages"),
    Page("mdn", "MDN Web Docs (Canvas API tutorial)", "document",
         "https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API/Tutorial/Pixel_manipulation_with_canvas",
         True, True, False, "Scroll the article to the end, and back; the live examples run"),
    Page("threejs", "Three.js (WebGPU compute particles)", "application",
         "https://threejs.org/examples/webgpu_compute_particles.html",
         True, True, False, "The simulation runs; the pointer moves across the canvas"),
    Page("figma", "Figma (design editor canvas)", "application",
         "https://www.figma.com/make/uulSQtKZ5FbRgWPRc0EmVo/Doodle-Fonts--Community-?p=f",
         False, False, True, "Pan and zoom the canvas"),
    Page("vscode", "VS Code for the Web (browser IDE)", "application", "https://vscode.dev/",
         True, True, False, "Open a new untitled file from the command palette and type twenty lines"),
    Page("prism", "Prism (document editing)", "application",
         "https://prism.openai.com/?pg=1&m=MIT-thesis-template%2FMIT-Thesis.tex",
         False, False, True, "Type a paragraph in the editor and wait for the preview to update"),
]

BY_IDENT = {page.ident: page for page in PAGES}


def full_urls(path):
    """{ident: URL} from the owner's list in scratch/, in corpus order, every parameter intact."""
    with open(path) as f:
        urls = [line.strip() for line in f if line.strip() and not line.lstrip().startswith("#")]
    if len(urls) != len(PAGES):
        raise ValueError(f"{path}: expected {len(PAGES)} URLs, found {len(urls)}")
    for page, url in zip(PAGES, urls):
        if url.split("?")[0] != page.url.split("?")[0]:
            raise ValueError(f"{path}: {url} doesn't match {page.ident}")
    return {page.ident: url for page, url in zip(PAGES, urls)}
