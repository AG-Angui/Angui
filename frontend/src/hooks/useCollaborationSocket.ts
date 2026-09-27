import { useEffect, useRef } from "react";
import type { SpaceEvent } from "../api/collaborationSpaces";

/** Maintains a best-effort realtime stream; HTTP event compensation remains the source of truth. */
export function useCollaborationSocket(token: string, spaceId: string, afterVersion: number, onEvents: (events: SpaceEvent[]) => void) {
  const callback = useRef(onEvents);
  callback.current = onEvents;
  useEffect(() => {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const socket = new WebSocket(`${protocol}//${window.location.host}/api/collaboration-spaces/${encodeURIComponent(spaceId)}/events/ws?access_token=${encodeURIComponent(token)}`);
    socket.onopen = () => socket.send("ping");
    socket.onmessage = (event) => {
      try {
        const value: unknown = JSON.parse(event.data as string);
        if (Array.isArray(value)) callback.current(value as SpaceEvent[]);
      } catch { /* HTTP compensation will recover malformed or missed frames. */ }
    };
    return () => socket.close();
  }, [spaceId, token]);
  void afterVersion;
}
