# Twenty Questions

[![CI](https://github.com/wpm/twentyquestions/actions/workflows/ci.yml/badge.svg)](https://github.com/wpm/twentyquestions/actions/workflows/ci.yml)

A desktop app that stages the classic parlor game Twenty Questions as a round‑table of LLM‑backed entities. One entity plays the **host**, who knows the secret object. The others play **players**, who must discover it using only yes‑or‑no questions. Everyone is an LLM; the human just sets the object, picks how many players take part, and watches the conversation unfold.

## Playing a game

Open the app, type an object into the toolbar (e.g. *elephant*, *wine glass*, *umbrella*), choose the number of players (1–10), and press **Start**. The chat panel streams the conversation in real time, with each speaker rendered in their own color. Press **Stop** at any point to end the round, or let the host declare the game over once a player lands on the answer.

Settings (NATS broker URL, Claude model) live behind a dedicated settings modal. The topic — the shared channel every entity talks on — lives on the toolbar and can be changed per game.

## How entities confer

Every entity, host and players alike, publishes and subscribes to the same topic on a NATS broker. Every message any one of them speaks is broadcast to all the others. Each entity keeps a rolling transcript of what has been said, timestamps and all, and uses that shared record as the basis for its reasoning. There is no private channel, no hidden whisper, no vote — the players confer simply by reading what everyone else has already said and replying to the room.

This means players can build on one another's questions, notice when a teammate is chasing a dead end, propose strategies mid‑round ("let's nail down whether it's alive first"), and collectively converge on a guess. The host can read the same transcript to decide when the group has stalled and needs a nudge, or when someone's phrasing is close enough to count as a correct guess.

## How entities decide whether to speak or remain silent

Turn‑taking is not scheduled. No entity is told *it is your turn now*. Instead, each one is driven by two triggers:

- **A new message arrives on the topic**, meaning somebody else just spoke.
- **An idle timeout elapses** with nothing said, meaning the room has gone quiet.

On either trigger, the entity wakes up, reads the full transcript, and asks its underlying Claude model a single question: *given everything that has been said, should I speak now, and if so what should I say?* The model answers by either calling a `speak` tool — in which case the message goes out on the topic for everyone to see — or by staying silent and ending its turn.

The system prompts steer that judgment. A **player** is told to speak when it has a useful new question, when a teammate is heading somewhere unproductive, when it wants to float a strategy or hypothesis, when it is confident enough to declare a guess, or when the conversation has fallen quiet — and to stay silent when another entity has just spoken and is likely to be answered by someone else, when its own question has already been asked, or when there is nothing yet worth adding. A **host** is told to speak when a question needs an answer, when a guess has been made, or when momentum has clearly stalled — and to stay silent otherwise. The host is also the only entity empowered to end the game, which it does by calling a `leave_game` tool once a player has guessed correctly.

Because every entity makes this decision independently and asynchronously, the flow of conversation is emergent: sometimes two players pile in at once, sometimes the room waits a beat for the host, sometimes a quiet player breaks a lull with a fresh angle. The structure of the game comes entirely out of each participant's own reading of the room.

## Tech stack

Tauri + Leptos desktop app, Rust throughout, NATS as the message bus between entities, and the Anthropic Claude API (Opus, Sonnet, or Haiku — selectable in settings) as the brain behind each host and player. An `ANTHROPIC_API_KEY` must be set in the environment for entities to think.
