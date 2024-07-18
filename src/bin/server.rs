use tokio::net::{TcpListener,TcpStream};
use tokio::io::{self,AsyncReadExt,AsyncWriteExt};
use core::fmt;
use std::fmt::{format, write};
use std::sync::mpsc::{Receiver, Sender};
use std::{error, string};
use std::{collections::HashMap, io::BufReader, sync::Arc};
use tokio::sync::{Mutex,broadcast};
use std::net::SocketAddr;

type db=Arc<Mutex<HashMap<String,String>>>;

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


#[tokio::main]
async fn main(){

    let Active_Users:db= Arc::new(Mutex::new(HashMap::new())); 

    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    let (tx,_)=broadcast::channel(20);

    println!("Server listening on port 8080");

    while let Ok((stream,addr)) = listener.accept().await {
        println!("Connection accepted! from {:?}",addr);
        let db=Arc::clone(&Active_Users);
        let rx=tx.subscribe();
        let tx1=tx.clone();
        tokio::spawn(async   move{
        handle_connection(stream, db,rx,tx1).await;
        });
    }


}

async fn handle_connection( stream:TcpStream,db:db,mut rx:tokio::sync::broadcast::Receiver<String>,tx1:tokio::sync::broadcast::Sender<String> ){
    let addr = stream.peer_addr().unwrap().to_string();
    let mut user:User=User { name: "".to_string(), addr:addr.clone() };
    let stream=Arc::new(Mutex::new(stream));
    let stream2=Arc::clone(&stream);
    tokio::spawn(async move {
        while let Ok(s)=rx.recv().await{
           if s=="closed"{
            break;
           }
           else{
            stream2.lock().await.write_all(s.as_bytes()).await.unwrap();
           }
           
        }
    });
        let mut login=false;
            
            
    let mut buf= vec![0;1024];
    loop{
        let mut guard = stream.lock().await;
            // Split the TcpStream while it is locked
            let (mut rd, mut wrt) = io::split(&mut *guard);
        match rd.read(& mut buf).await{
            Ok(n) if n==0 =>{
                println!("Connection closed !");
                tx1.send("closed".to_string()).unwrap();
                removeKey(& mut user, Arc::clone(&db)).await;
                break;
            }
            Ok(n) =>{
                let Message =String::from_utf8_lossy(& buf).to_string();
                let parts:Vec<&str>=Message.split(";").collect();
                match parts[0]{
                    "login"=>{
                        if login == true{
                            wrt.write_all("Already logged in".as_bytes()).await.unwrap();
                            continue;
                        }
                        match handle_login(&addr,parts,Arc::clone(&db)).await{
                            Ok(s)=>{
                                println!("User logged in :{s}");
                                login=true;
                                user=User{
                                    name: s.clone(),
                                    addr:addr.clone()
                                };
                               wrt.write_all("success".as_bytes()).await.unwrap();
                               let s= format!("{s}:joined the chat");
                               tx1.send(s.to_string()).unwrap();
                            }
                            Err(_) =>{
                            }
                        }
                    }
                    "getUser"=>{
                       match handle_get(parts,Arc::clone(&db)).await{
                        Ok(s)=>{
                           wrt.write_all(s.as_bytes()).await.unwrap();
                        }
                        Err(_)=>{
                            wrt.write_all("UserDown".as_bytes()).await.unwrap();
                        }
                       }
                    }
                    _=>{
                        wrt.write_all("invalid request".as_bytes()).await.unwrap();
                    }
                }
                buf.clear();
                buf.resize(1024, 0);

            }
            Err(e)=>{
                println!("Error occured during the reading the stream {:?}",e);
                tx1.send("closed".to_string()).unwrap();
                removeKey(& mut user, Arc::clone(&db)).await;
                break;
            }

        }
    }
}

 async fn handle_login(addr:&str,parts:Vec<&str>,db:db)-> Result<String,CustomError>{
    let name=parts[1].to_string();
    let mut lock=db.lock().await;
    match lock.get(&name){
            Some(n)=>{
                Err(CustomError::UserExists)
            }
            None =>{
                let addr=format!("{:?}",addr);
                lock.insert(name.clone(),addr);
                Ok(name)           
            }
        }
}

async fn removeKey(User : & mut User,db: db){
    let mut  db= db.lock().await;
    let name = & User.name;
    let _=db.remove(name);
}
async fn handle_get(parts:Vec<&str>,db:db)->Result<String,CustomError>{
   
   let req = parts[1].to_string();
   let mut lock=db.lock().await;
   match lock.get(&req){
    Some(s)=>{
        match check_status(&s).await{
            Ok(_)=>{
                let res= format!("User;{};{}",req,s);
                Ok(res)
            }
            Err(e)=>{
                lock.remove(&req);
                Err(CustomError::UserDown)
            }
        }

    }
    None=>{
        Err(CustomError::UserIsNotOnline)
    }
   }
    // send user details
}

async fn check_status(addr:&str) -> Result<(),io::Error>{
    match TcpStream::connect(addr).await{
        Ok(_)=>{
            Ok(())
        }
        Err(e)=>{
            Err(e)
        }
    }
}

