FROM node:14.17.4

WORKDIR /usr/src/app
COPY package*.json ./
COPY yarn* ./
COPY *.ts ./

RUN yarn install

ENTRYPOINT ["yarn", "start"]
CMD []
