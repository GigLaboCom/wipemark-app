# Quarterly update from the apps team

This quarter was mostly about stability. We closed more bugs than we opened for the first time in a year, and the crash rate on launch fell by half after we moved the database migration off the main thread.​

The new onboarding flow shipped to everyone in August. Fewer people now drop out before the first sync, although the welcome tour is still longer than it needs to be, and we plan to cut it to two screens.

Search got faster in large folders. The index is now built in the background and kept up to date as files change, so the first search after a restart no longer stalls the window for several seconds.

We also spent time on accessibility. Every control in the settings window can now be reached with the keyboard, and the colours in the dark theme pass the contrast checks we set ourselves in the spring.

Not everything went to plan. The export redesign slipped to next quarter because the file picker behaved differently on each platform, and we would rather ship it late than ship it twice.

Next quarter we want to finish the export work, make sync conflicts easier to under­stand, and spend a week on nothing but the small annoyances people keep writing to us about.

Thanks to everyone who filed a bug, answered a survey or sat through a test session with us. It made a real difference.
