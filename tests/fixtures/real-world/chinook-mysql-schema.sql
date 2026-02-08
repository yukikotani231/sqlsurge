-- Chinook Database Schema (MySQL version with AUTO_INCREMENT PKs)
-- Source: https://github.com/lerocha/chinook-database
-- License: MIT License
-- Original author: Luis Rocha

CREATE TABLE Artist (
    ArtistId INT NOT NULL AUTO_INCREMENT,
    Name NVARCHAR(120),
    CONSTRAINT PK_Artist PRIMARY KEY (ArtistId)
);

CREATE TABLE Album (
    AlbumId INT NOT NULL AUTO_INCREMENT,
    Title NVARCHAR(160) NOT NULL,
    ArtistId INT NOT NULL,
    CONSTRAINT PK_Album PRIMARY KEY (AlbumId),
    CONSTRAINT FK_AlbumArtistId FOREIGN KEY (ArtistId) REFERENCES Artist (ArtistId)
);

CREATE TABLE Employee (
    EmployeeId INT NOT NULL AUTO_INCREMENT,
    LastName NVARCHAR(20) NOT NULL,
    FirstName NVARCHAR(20) NOT NULL,
    Title NVARCHAR(30),
    ReportsTo INT,
    BirthDate DATETIME,
    HireDate DATETIME,
    Address NVARCHAR(70),
    City NVARCHAR(40),
    State NVARCHAR(40),
    Country NVARCHAR(40),
    PostalCode NVARCHAR(10),
    Phone NVARCHAR(24),
    Fax NVARCHAR(24),
    Email NVARCHAR(60),
    CONSTRAINT PK_Employee PRIMARY KEY (EmployeeId),
    CONSTRAINT FK_EmployeeReportsTo FOREIGN KEY (ReportsTo) REFERENCES Employee (EmployeeId)
);

CREATE TABLE Customer (
    CustomerId INT NOT NULL AUTO_INCREMENT,
    FirstName NVARCHAR(40) NOT NULL,
    LastName NVARCHAR(20) NOT NULL,
    Company NVARCHAR(80),
    Address NVARCHAR(70),
    City NVARCHAR(40),
    State NVARCHAR(40),
    Country NVARCHAR(40),
    PostalCode NVARCHAR(10),
    Phone NVARCHAR(24),
    Fax NVARCHAR(24),
    Email NVARCHAR(60) NOT NULL,
    SupportRepId INT,
    CONSTRAINT PK_Customer PRIMARY KEY (CustomerId),
    CONSTRAINT FK_CustomerSupportRepId FOREIGN KEY (SupportRepId) REFERENCES Employee (EmployeeId)
);

CREATE TABLE Genre (
    GenreId INT NOT NULL AUTO_INCREMENT,
    Name NVARCHAR(120),
    CONSTRAINT PK_Genre PRIMARY KEY (GenreId)
);

CREATE TABLE MediaType (
    MediaTypeId INT NOT NULL AUTO_INCREMENT,
    Name NVARCHAR(120),
    CONSTRAINT PK_MediaType PRIMARY KEY (MediaTypeId)
);

CREATE TABLE Track (
    TrackId INT NOT NULL AUTO_INCREMENT,
    Name NVARCHAR(200) NOT NULL,
    AlbumId INT,
    MediaTypeId INT NOT NULL,
    GenreId INT,
    Composer NVARCHAR(220),
    Milliseconds INT NOT NULL,
    Bytes INT,
    UnitPrice NUMERIC(10,2) NOT NULL,
    CONSTRAINT PK_Track PRIMARY KEY (TrackId),
    CONSTRAINT FK_TrackAlbumId FOREIGN KEY (AlbumId) REFERENCES Album (AlbumId),
    CONSTRAINT FK_TrackMediaTypeId FOREIGN KEY (MediaTypeId) REFERENCES MediaType (MediaTypeId),
    CONSTRAINT FK_TrackGenreId FOREIGN KEY (GenreId) REFERENCES Genre (GenreId)
);

CREATE TABLE Invoice (
    InvoiceId INT NOT NULL AUTO_INCREMENT,
    CustomerId INT NOT NULL,
    InvoiceDate DATETIME NOT NULL,
    BillingAddress NVARCHAR(70),
    BillingCity NVARCHAR(40),
    BillingState NVARCHAR(40),
    BillingCountry NVARCHAR(40),
    BillingPostalCode NVARCHAR(10),
    Total NUMERIC(10,2) NOT NULL,
    CONSTRAINT PK_Invoice PRIMARY KEY (InvoiceId),
    CONSTRAINT FK_InvoiceCustomerId FOREIGN KEY (CustomerId) REFERENCES Customer (CustomerId)
);

CREATE TABLE InvoiceLine (
    InvoiceLineId INT NOT NULL AUTO_INCREMENT,
    InvoiceId INT NOT NULL,
    TrackId INT NOT NULL,
    UnitPrice NUMERIC(10,2) NOT NULL,
    Quantity INT NOT NULL,
    CONSTRAINT PK_InvoiceLine PRIMARY KEY (InvoiceLineId),
    CONSTRAINT FK_InvoiceLineInvoiceId FOREIGN KEY (InvoiceId) REFERENCES Invoice (InvoiceId),
    CONSTRAINT FK_InvoiceLineTrackId FOREIGN KEY (TrackId) REFERENCES Track (TrackId)
);

CREATE TABLE Playlist (
    PlaylistId INT NOT NULL AUTO_INCREMENT,
    Name NVARCHAR(120),
    CONSTRAINT PK_Playlist PRIMARY KEY (PlaylistId)
);

CREATE TABLE PlaylistTrack (
    PlaylistId INT NOT NULL,
    TrackId INT NOT NULL,
    CONSTRAINT PK_PlaylistTrack PRIMARY KEY (PlaylistId, TrackId),
    CONSTRAINT FK_PlaylistTrackPlaylistId FOREIGN KEY (PlaylistId) REFERENCES Playlist (PlaylistId),
    CONSTRAINT FK_PlaylistTrackTrackId FOREIGN KEY (TrackId) REFERENCES Track (TrackId)
);
