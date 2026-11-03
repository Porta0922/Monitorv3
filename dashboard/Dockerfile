# Stage 1: Build the React application
FROM node:22-alpine@sha256:fe4f6d74e20370acb04ad889077d14d0f64a9c8ea6a42bc60cddf1e844699914 AS build

WORKDIR /app

# Copy package files and install dependencies
COPY package.json package-lock.json* ./
RUN npm ci

# Copy all source files and build
COPY . .
RUN npm run build

# Stage 2: Serve the application with Nginx
FROM nginx:alpine@sha256:a3554c0d0b48177b1413c26e01f695334428020d9df01243fbae3e11cfd17417 AS runtime

# Copy built assets from the builder stage
COPY --from=build /app/dist /usr/share/nginx/html

# Serve the SPA and proxy API requests on the same origin.
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]

FROM runtime AS production
COPY nginx.production.conf /etc/nginx/nginx.production.conf
USER 65532:65532
EXPOSE 8443
ENTRYPOINT ["nginx", "-c", "/etc/nginx/nginx.production.conf"]
CMD ["-g", "daemon off;"]
FROM runtime AS default
