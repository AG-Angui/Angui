export type ParsedCoordinatePair =
  | {
      ok: true;
      longitude: number | null;
      latitude: number | null;
    }
  | {
      ok: false;
      message: string;
    };

export function parseOptionalCoordinatePair(
  longitudeText: string,
  latitudeText: string,
): ParsedCoordinatePair {
  const longitudeValue = longitudeText.trim();
  const latitudeValue = latitudeText.trim();

  if (!longitudeValue && !latitudeValue) {
    return { ok: true, longitude: null, latitude: null };
  }

  if (!longitudeValue || !latitudeValue) {
    return {
      ok: false,
      message: "经度和纬度必须同时填写或同时留空。",
    };
  }

  const longitude = Number(longitudeValue);
  const latitude = Number(latitudeValue);

  if (!Number.isFinite(longitude) || !Number.isFinite(latitude)) {
    return { ok: false, message: "经度和纬度必须是有效数字。" };
  }

  if (longitude < -180 || longitude > 180) {
    return { ok: false, message: "经度必须在 -180 到 180 之间。" };
  }

  if (latitude < -90 || latitude > 90) {
    return { ok: false, message: "纬度必须在 -90 到 90 之间。" };
  }

  return { ok: true, longitude, latitude };
}
