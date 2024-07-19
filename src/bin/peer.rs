use bytes::buf::Writer;
use futures_util::stream::{SplitSink,SplitStream};
use tokio::net::{TcpListener,TcpStream};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::sync::futures::Notified;
use tokio_tungstenite::tungstenite::{buffer, Message};
use tokio_tungstenite::MaybeTlsStream;
use core::{fmt, task};
use std::fmt::{format, write};
use std::ops::Not;
use std::process;
use std::str::Utf8Error;
use tokio_tungstenite::{accept_async, connect_async};
use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::WebSocketStream;
use tokio::sync::mpsc::{self, Receiver,Sender};
use std::{error, string, vec};
use std::{collections::HashMap, io::BufReader, sync::Arc};
use tokio::sync::{Mutex};
use crossterm::{cursor,
    terminal::{Clear,ClearType},
    ExecutableCommand};
use std::io::{stdout, Stdout, Write};  

#[derive(Debug)]
struct User{
    name:String,
    addr:String,
    // more fields
}
#[derive(Debug)]
enum CustomError{
    UserExists,
    UserIsNotOnline,
    UserDown,

}
impl fmt::Display for CustomError{
    fn fmt(&self, f: & mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CustomError::UserExists => write!(f, "User already exists"),
            CustomError::UserDown=>write!(f, "User is Unreachable"),
            CustomError::UserIsNotOnline => write!(f, "User is not active")
        }
    }
}
impl std::error::Error for CustomError{}

#[derive(Debug)]
struct Nofication{
    from:String,
    Message:String,
}

type NF = Arc<Mutex<Vec<Nofication>>>;
type WH= Arc<Mutex<HashMap<String,StreamType>>>;


enum StreamType {
    TcpWrite(WriteHalf<TcpStream>),
    WsSend(SplitSink<WebSocketStream<TcpStream>, Message>),
    WsSendTls(SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>),
}

#[tokio::main]
async fn main(){
    // this is listen upcoming connections from other peers and recieve messages
    let mut listener= TcpListener::bind("127.0.0.1:8010").await.unwrap();

    // connection to the server
    let mut stream= TcpStream::connect("127.0.0.1:8080").await.unwrap();
    
    // creating the channels to transmit the data between the tasks with buffer capacity 10
    let (sender,receiver)= mpsc::channel::<String>(30);

    // buffer to read the response from the server
    let mut buf = vec![0;128];

    // This vector holds the Notifications recieved from the server and stores the messages sent by other clientts 
    let mut v:Vec<Nofication>=Vec::new();

    // this holds the write half of the stream when we need to send the message or sent server the requests
    let mut h:HashMap<String,StreamType>= HashMap::new();

    // data types to transfer the access between the tasks
    let mut write_handlers=Arc::new(Mutex::new(h));
    
    // NOtifications 
    let mut Notifications:NF=Arc::new(Mutex::new(v));

    // cloning the sender to send the messages to receiving thread and parsing the message whether its recieved from the client or server

    let s=sender.clone();

    // this function handles the listener to listen for incoming connections
    handle_listener(listener,Arc::clone(& write_handlers),s).await;


    // storing the user name
    let mut user_name=Arc::new(Mutex::new(String::new()));

    // this function handles the input from the user
    // this function handle channels that are shared for different spawned tasks
    handle_channels(receiver,Arc::clone(& Notifications),Arc::clone(& user_name),sender.clone(),Arc::clone(&write_handlers)).await;


    let s=sender.clone();
    


    // this handles the login of the user. ON success it will continues to take the input from the user and parse it 
    match handle_login(stream,&mut buf,Arc::clone(& write_handlers),s,Arc::clone(&user_name)).await{
        Ok(s)=>{
            // it handles the input
            handle_input(Arc::clone(&Notifications),Arc::clone(& write_handlers)).await;
        }
        Err(_)=>{
            println!("error while connecting to the server");
            process::exit(1);
        }
    } 
}

async fn handle_listener(listener:TcpListener,write_handler:WH,sender:Sender<String>){
    
    // spawning new thread to accept new connections and again spawning new threads to listnen for upcoming messages from the client
    tokio::spawn(async move{
        while let Ok((stream,_))=listener.accept().await{

            // this upgrade the stream to websocket connection
            let ws_stream= accept_async(stream).await.expect("Error while hand shake");

            // spliting the stream into readhalf and writehalf
            let (mut wrt,mut rd)=ws_stream.split();

            // reading the nest string to get name of the peer
            let name=rd.next().await.unwrap().unwrap().to_string();

            // storing the name and write half of the client
            let mut write_handler=write_handler.lock().await;

            write_handler.insert(name.clone(),StreamType::WsSend(wrt));

            // clonning the sender to avoid "use of moved value here"
            let sender=sender.clone();

            // spawning new tasks to read the upcoming Messages from users connections
            tokio::spawn(async move{
                let sender=sender;
            loop{
                // reading the next message 
                match rd.next().await{
                    // matching the message
                    Some(msg)=>{
                       match msg{
                        // if it is ok then send the recived message to reciver
                        Ok(Message::Text(s))=>{

                            sender.send(format!("{name};{s}")).await.unwrap();
                        }

                        // if it is error then close the connection the break this task
                        Err(e)=>{
                            println!("Error while reading message from client: {:?}",e);
                            sender.send(format!("{};down",name)).await.unwrap();
                            break;
                        }

                        // if the recived message is another message just print the messages
                        _=>{
                            sender.send(format!("{};invalid response",name)).await.unwrap();
                        }
                       }
                    }

                    // or if client closes the then print the message
                    None=>{
                        sender.send(format!("{};down",name)).await.unwrap();
                        break;
                    }
                }
            }
            });
        }
    });
}


// this function handles channels
async fn handle_channels(mut receiver:Receiver<String>,Notifications:NF,user_name:Arc<Mutex<String>>,mut sender:Sender<String> ,write_handler:WH){

    // spawing a task for reciever
    tokio::spawn(async move{
        loop{
            match receiver.recv().await{
                // it match the message 
                Some(s)=>{
                    if s.contains(";"){
                        let  v:Vec<&str>=s.split(";").collect();
                        let mut n= Notifications.lock().await;
                        if(v[0]=="User"){
                           match Connect_User(& v,Arc::clone(&Notifications),Arc::clone(& user_name),sender.clone(),Arc::clone(& write_handler)).await{
                            Ok(_)=>{}
                            Err(e)=>{
                                println!("Error while connecting to user");
                            }
                           }
                            let s=Nofication{
                                from:v[1].to_string(),
                                Message:v[2].to_string(),
                            };
                            n.push(s);
                        }

                        else{   let s=Nofication{
                                from:v[0].to_string(),
                                Message:v[1].to_string(),
                            };
                            n.push(s);
                        }
                        }  
                }
                None=>{
                    println!("No Notifications");
                }
            }
        }
    });
}

// connect new Users

async fn Connect_User(v:& Vec<&str>,n:NF,user_name:Arc<Mutex<String>>,mut sender:Sender<String>,write_handler:WH)->Result<(),tokio_tungstenite::tungstenite::Error>{

    let s=format!("ws//:{}",v[2]);
    let (mut stream,_) = connect_async(s).await.unwrap();
    let (mut wrt_stream, mut rd_stream) =stream.split();
    let user_name=user_name.lock().await.to_string();
    wrt_stream.send(Message::text(user_name)).await.unwrap();
    let client_name=v[1].to_string();
    tokio::spawn(async move{
        let client_name=client_name;
        loop{
            match rd_stream.next().await.unwrap(){
                Ok(m)=>{
                    match m {
                        Message::Text(s)=>{
                            sender.send(format!("{};s",client_name)).await.unwrap();
                        }
                        _=>{
                            sender.send(format!("{};invalid response",client_name)).await.unwrap();
                        }
                    }
                }
                Err(e)=>{
                    sender.send(format!("{};down",client_name)).await.unwrap();
                }
            }
        }
    }            
    );

    let mut wh=write_handler.lock().await;
    wh.insert(v[1].to_string().clone(),StreamType::WsSendTls(wrt_stream));

    println!("{} is connected !",v[1]);
    
    Ok(())
}

// it handles the login of the User
async fn handle_login(stream:TcpStream,buf:& mut Vec<u8>,write_handler:WH,sender:Sender<String>,mut name:Arc<Mutex<String>>)-> Result<bool,std::io::Error>{
    // input
    let mut input=String::new();

    // spliting the stream of server
    let (mut rd,mut  wrt) = io::split(stream);

    // starting a loop until we get login successful
    loop{
        // Split the TcpStream while it is locked
    println!("Enter user name to login into the chat server !");

    // entering the name
    std::io::stdin().read_line(& mut input).unwrap();

    // formats the input
    let req=format!("login;{}",input.trim());
   
    // sends the login request 
    match wrt.write_all(req.as_bytes()).await{
        Ok(_)=>{

        }
        // if error then return error
        Err(e)=>{
            println!("error while sending data to the server ,{}",e);
            return Err(e);
        }
    }

    // reads the response
    match rd.read(buf).await{
        Ok(n)=>{
            let res= String::from_utf8_lossy(&buf[..n]).to_string().trim().to_string();
            let res=res.as_str();
            match res{
                // on success it spawn threads to recieve messages from the server
                "success"=>{
                    println!("Login Successful");
                    spawn_threads(rd, wrt,write_handler,sender).await;
                    { let mut name=name.lock().await;
                    *name=input.to_string();
                    }
                    return Ok(true);
                }
                "Already logged in"=>{
                    println!("Username already exists");
                }
                _=>{
                    println!("Invalid username");
                }
            }
              
        }
        // returns errargsor if there was error
        Err(e) =>{
            println!("Connection failed");
            return Err(e)
        }
    }
        // clears the buffer
        buf.clear();
        buf.resize(1024,0);
    }
}

// this function spawn threads 
async fn spawn_threads(mut rd:ReadHalf<TcpStream>,mut wrt: WriteHalf<TcpStream>,write_handler:WH,sender:Sender<String>){

        // spwaing task for receiving messages from the server
        tokio::spawn(async move{

            // read buffer
            let mut buf= vec![0;64];
            loop{

                // reading the response
                match rd.read(& mut buf).await{

                    // if it is ok then forwarding the message for reciver
                    Ok(n)=>{
                        if(n==0) {break;}
                        let s= String::from_utf8_lossy(& buf[..n]).to_string();
                        sender.send(format!("0;{s}")).await.unwrap();

                        // clearing and resizing the buffer
                        buf.clear();
                        buf.resize(64, 0);
                    }

                    // breaking if erro
                    Err(e)=>{
                        println!("Error whilw reading from the sender");
                        break;
                    }
                }
            }
            });

        // entering the write half of the stream into sever 

        // server is represented by "0"
        let mut write_handler= write_handler.lock().await;
        write_handler.insert("0".to_string(), StreamType::TcpWrite(wrt));



}

// printing the starters for users
fn print_starters(stdout: &mut Stdout){
    stdout.execute(Clear(ClearType::All)).unwrap();
    stdout.execute(cursor::MoveTo(0,0)).unwrap();
    println!("1:Notifications");
    println!("2:Type user name : msg to send the Message");
    println!("3.Type get : user name to connect the address");
    println!("0:Exit");
}

// handling the input
async fn handle_input(Notifications:NF,write_handler:WH){
    let Notifications=Notifications;
    let write_handler=write_handler;
    let mut stdout=stdout();
    let mut input = String::new();
    loop{
        print_starters(& mut stdout);
        std::io::stdin().read_line(& mut input).expect("failed to read the line");
        input=input.trim().to_string();
        parse_input(&input,Arc::clone(& Notifications),Arc::clone(& write_handler)).await;
    }
}

// parsing the input
async fn parse_input(input: & str,Notifications:NF,write_handler:WH){
    match input{
        "0"=>{
            process::exit(0);
        }
        _=>{
            if (input.starts_with("1")){

                print_Notifications(Notifications).await;
            }
            else if(input.starts_with("2")){
               let v= input.split(":").collect();
               send_message(&v,write_handler).await; 
            }
            else if(input.starts_with("3")){

            }
        }
    }

}

async fn print_Notifications(Notifications:NF){
    let  Notifications= Notifications.lock().await;
    for n in &*Notifications{
        println!("From: {} Message: {}",n.from,n.Message);
    }
}

async fn send_message(v:&Vec<& str>,write_handler:WH){
    let write_handler=write_handler.lock().await;

    match write_handler.get(v[1]){
        Some(s)=>{
            match s{
                StreamType::TcpWrite(s)=>{
                    println!("this is server");
                }
                StreamType::WsSend(s)=>{
                    s.send(Message::Text(v[2].to_string())).await.unwrap();
                }   
                StreamType::WsSendTls(s)=>{

                }
            }
        }
        None=>{
            println!("{} is not found",v[1]);
        }
    }
}