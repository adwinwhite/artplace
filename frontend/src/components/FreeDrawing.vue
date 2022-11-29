<script setup>
import { ref, onMounted, onBeforeUpdate } from 'vue';
import { fabric } from 'fabric';
import * as protobuf from 'protobufjs';
import { Buffer } from 'buffer';
import Long from 'long';

let brushColor = ref('#000000');
let brushWidth = ref(2);
let myroom = ref('');

let initDone = false;
let isMouseDown = false;
/** @type {WebSocket | null} */
let socket = null;
let myid = null;
/* let myroom = null; */
let screenCanvas = null;
let points = [];

// id -> fabric.brush.
let playersBrushes = new Map();
let brushStack = [];

let ClientMessage = null;
let InitClient = null;
// let Uid = null;
let JoinRoom = null;
let Brush = null;
let PencilBrush = null;
let SetBrush = null;
let Movement = null;
let Pos = null;


function saveBrush(canvas) {
  brushStack.push(canvas.freeDrawingBrush);
}

function restoreBrush(canvas) {
  canvas.freeDrawingBrush = brushStack.pop();
}

function kind2brush(kind) {
  var brush = null;
  if (kind.pencil) {
    brush = new fabric.PencilBrush(screenCanvas);
    brush.color = kind.pencil.color;
    brush.width = kind.pencil.width;
    brush.kind = kind;
  }
  return brush;
}

function createDefaultBrush(canvas) {
  const kind = {
    pencil: {
      width: 1,
      color: "#000000"
    }
  }
  const brush = kind2brush(kind);
  return brush;
}

function getBrush(id) {
  let brush = playersBrushes.get(id);
  if (!brush) {
    brush = createDefaultBrush(screenCanvas);
    playersBrushes.set(id, brush);
  }
  return brush;
}


function onBrushColorChange(event) {
  const setBrush = {
    id: myid,
    brush: {
      pencil: {
        width: getBrush(myid).width,
        color: brushColor.value,
      },
    },
  };
  const clientMessage = {
    setBrush: setBrush,
  };
  socket.send(
    ClientMessage.encode(ClientMessage.create(clientMessage)).finish()
  );

  const brush = kind2brush(setBrush.brush);
  playersBrushes.set(myid, brush);
  screenCanvas.freeDrawingBrush = brush;
}

function onBrushWidthChange(event) {
  const setBrush = {
    id: myid,
    brush: {
      pencil: {
        width: parseInt(brushWidth.value),
        color: getBrush(myid).color,
      },
    },
  };
  const clientMessage = {
    setBrush: setBrush,
  };
  socket.send(
    ClientMessage.encode(ClientMessage.create(clientMessage)).finish()
  );

  const brush = kind2brush(setBrush.brush);
  playersBrushes.set(myid, brush);
  screenCanvas.freeDrawingBrush = brush;
}

function onRoomChange(event) {
  const joinRoom = {
    id: myid,
    room: event.target.value,
  };
  const clientMessage = {
    joinRoom: joinRoom,
  };
  socket.send(
    ClientMessage.encode(ClientMessage.create(clientMessage)).finish()
  );
}

function connectWs() {
  // const { location } = window

  // const proto = location.protocol.startsWith('https') ? 'wss' : 'ws'
  // const wsUri = `${proto}://${location.host}/ws`
  const wsUri = 'wss://www.adwin.icu/ws';

  console.log('Connecting...');
  socket = new WebSocket(wsUri);

  socket.onopen = () => {
    console.log('Connected');
  };

  socket.onmessage = async (ev) => {
    ev.data
      .arrayBuffer()
      .then((buf) => new Uint8Array(buf))
      .then((bytes) => {
        const clientMsg = ClientMessage.decode(bytes);
        console.log('Received client message: ');
        console.log(clientMsg);
        handleClientMessage(clientMsg);
      })
      .catch((error) => {
        // ...handle/report error...
        console.log(error);
      });
  };

  socket.onclose = () => {
    console.log('Disconnected');
    socket = null;
  };
}

function initProtobuf() {
  protobuf.util.Long = Long;
  let protoFile = './messages.proto';
  protobuf.load(protoFile, function (err, root) {
    if (err) throw err;

    // Obtain a message type
    ClientMessage = root.lookupType('artplace.messages.ClientMessage');
    InitClient = root.lookupType('artplace.messages.InitClient');
    // Uid = root.lookupType('artplace.messages.Uid');
    Brush = root.lookupType('artplace.messages.Brush');
    PencilBrush = root.lookupType('artplace.messages.PencilBrush');
    SetBrush = root.lookupType('artplace.messages.SetBrush');
    Movement = root.lookupType('artplace.messages.Movement');
    JoinRoom = root.lookupType('artplace.messages.JoinRoom');
    Pos = root.lookupType('artplace.messages.Movement.Pos');
  });
}

function setBrush(canvas, brush) {
  canvas.isDrawingMode = true;
  canvas.freeDrawingBrush = brush;
}

let options = { e: { 
                  pointerId: 1,
                  /* isPrimary: true */
                }};

function pointDrawer(movement) {
  /* console.log('Drawing for someone using the brush: '); */
  /* console.log(brush); */
  saveBrush(screenCanvas);
  const brush = getBrush(movement.id);
  setBrush(brush.canvas, brush);
  if (movement.kind == 1) {
    brush.onMouseMove(movement.point, options);
  } else if (movement.kind == 0) {
    brush.onMouseDown(movement.point, options);
  } else if (movement.kind == 2) {
    console.log(options);
    brush.onMouseUp(options);
  }
  restoreBrush(screenCanvas);
}

function handleClientMessage(msg) {
  if (msg.initClient) {
    screenCanvas.remove(...screenCanvas.getObjects());
    screenCanvas.renderAll();
    playersBrushes = new Map();
    myid = msg.initClient.id;
    myroom.value = msg.initClient.room;
    console.log('I have id: ', myid);
    msg.initClient.messages.forEach((m) => handleClientMessage(m));

    // set my brush for this room
    const setBrush = {
      id: myid,
      brush: screenCanvas.freeDrawingBrush.kind 
    };
    const clientMessage = {
      setBrush: setBrush,
    };
    socket.send(
      ClientMessage.encode(ClientMessage.create(clientMessage)).finish()
    );

    // after initialization, allow drawing.
    initDone = true;
  } else if (msg.setBrush) {
    const brush = kind2brush(msg.setBrush.brush);
    playersBrushes.set(msg.setBrush.id, brush);
  } else if (msg.movement) {
    if (myid != msg.movement.id) {
      pointDrawer(msg.movement);
    }
  } else if (msg.joinRoom) {
  }
}

function sendMovement(options, moveKind) {
  let kind = null;
  if (moveKind == 'down') {
    kind = 0;
  } else if (moveKind == 'move') {
    kind = 1;
  } else if (moveKind == 'up') {
    kind = 2;
  }
  const movement = {
    id: myid,
    point: {
      x: options.pointer.x,
      y: options.pointer.y
    },
    kind: kind
  };
  const clientMessage = {
    movement: movement,
  };
  socket.send(
    ClientMessage.encode(ClientMessage.create(clientMessage)).finish()
  );
}

onMounted(() => {
  screenCanvas = new fabric.Canvas('can', {
    width: 400,
    height: 400,
    isDrawingMode: true,
  });
  screenCanvas.freeDrawingBrush = createDefaultBrush(screenCanvas);
  screenCanvas.on('mouse:down', function (options) {
    if (initDone) {
      isMouseDown = true;
      screenCanvas.isDrawingMode = true;

      // send message
      sendMovement(options, "down");
    }
  });
  screenCanvas.on('mouse:move', function (options) {
    if (initDone && isMouseDown) {
      // send message
      sendMovement(options, "move");
    }
  });
  screenCanvas.on('mouse:up', (options) => {
    if (initDone && isMouseDown) {
      // send message
      sendMovement(options, "up");
    }
    isMouseDown = false;
  });

  // // init protobuf and websocket.
  initProtobuf();
  connectWs();
});

onBeforeUpdate(() => {
  if (!socket) {
    connectWs();
  }
});
</script>

<template>
  <p>Can you see me?</p>
  <form onsubmit="return false">
    <input type="color" v-model="brushColor" @change="onBrushColorChange" />
    <label for="brush-width">Brush Width:</label>
    <input
      id="brush-width"
      v-model="brushWidth"
      type="range"
      min="1"
      max="100"
      @change="onBrushWidthChange"
    />
    <label for="room">Room:</label>
    <input
      id="room"
      type="text"
      :placeholder="myroom"
      maxlength="16"
      @change="onRoomChange"
    />
  </form>
  <canvas id="can" style="border: 1px solid #ccc"></canvas>
  <p>Where is my canvas?</p>
</template>

<style scoped></style>
