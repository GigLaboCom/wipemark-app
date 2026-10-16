// Wipemark — the site's only script. It does one thing: a screenshot or the
// video that is not there yet is shown as a labelled empty frame instead of a
// broken image. The page reads and works without it.
(function () {
  "use strict";

  function markShot(img) {
    var figure = img.closest(".shot");
    if (!figure || figure.classList.contains("is-missing")) return;
    figure.classList.add("is-missing");
    var frame = figure.querySelector(".shot-frame");
    if (!frame) return;
    var note = document.createElement("p");
    note.className = "missing-note";
    note.textContent = "Screenshot to come: " + (img.getAttribute("alt") || "");
    frame.appendChild(note);
  }

  document.querySelectorAll(".shot img").forEach(function (img) {
    if (img.complete && img.naturalWidth === 0) {
      markShot(img);
    } else {
      img.addEventListener("error", function () {
        markShot(img);
      });
    }
  });

  document.querySelectorAll(".video").forEach(function (figure) {
    var video = figure.querySelector("video");
    var source = figure.querySelector("video source");
    if (!video || !source) return;
    function missing() {
      if (figure.classList.contains("is-missing")) return;
      figure.classList.add("is-missing");
      var frame = figure.querySelector(".video-frame");
      if (!frame) return;
      var note = document.createElement("p");
      note.textContent = "The intro video is not published yet.";
      frame.appendChild(note);
    }
    source.addEventListener("error", missing);
    // With preload="none" nothing is fetched until a click, so ask once,
    // cheaply, whether the file is there at all.
    if (window.fetch) {
      fetch(source.getAttribute("src"), { method: "HEAD" })
        .then(function (response) {
          if (!response.ok) missing();
        })
        .catch(function () {});
    }
  });
})();
