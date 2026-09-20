import type { NextConfig } from "next";

const API = process.env.API_ORIGIN ?? "http://127.0.0.1:3000";

const nextConfig: NextConfig = {
  // Sortie autonome : l'image de production n'embarque que ce que le serveur
  // exécute réellement, sans node_modules ni sources.
  output: "standalone",

  // Le front et l'API partagent la même origine : en production c'est Nginx
  // qui distingue sur le chemin, en développement c'est ce proxy. Aucun CORS
  // nulle part, et le cookie de session reste SameSite=Strict.
  // Voir docs/adr/0008-deploiement-nginx-docker.md.
  async rewrites() {
    return [
      { source: "/api/:path*", destination: `${API}/api/:path*` },
    ];
  },
};

export default nextConfig;
