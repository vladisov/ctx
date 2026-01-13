import * as http from "http";

export function checkServerHealth(host: string, port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const req = http.get(`http://${host}:${port}/`, (res) => {
      resolve(res.statusCode === 200);
    });

    req.on("error", () => resolve(false));
    req.setTimeout(2000, () => {
      req.destroy();
      resolve(false);
    });
  });
}
